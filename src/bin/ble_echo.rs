//! Minimal BLE NUS echo service for hardware testing.
//!
//! Advertises as "BLE Echo", accepts any connection without pairing,
//! and reflects every byte written to the RX characteristic back on TX.
//!
//! Use `ble-serial` on Linux to get a PTY device, then open it with minicom:
//!   pip install ble-serial
//!   ble-scan                              # find the MAC
//!   ble-serial -d XX:XX:XX:XX:XX:XX      # creates /dev/ttyBLE (or similar)
//!   minicom -D /dev/ttyBLE
//!
//! No pairing or authentication is required.

#![no_std]
#![no_main]

use embassy_boot_nrf::{AlignedBuffer, BlockingFirmwareUpdater, FirmwareUpdaterConfig};
use embassy_executor::Spawner;
use embassy_nrf::config::HfclkSource;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::nvmc::Nvmc;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use nrf_sdc::SoftdeviceController;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use static_cell::StaticCell;
use trouble_host::prelude::*;
use {defmt_rtt as _, panic_probe as _};

use hello_graphics::board;
use hello_graphics::fw::ble::{NusServer, init_ble};
use hello_graphics::fw::device_id;

// ---------------------------------------------------------------------------
// Echo task
// ---------------------------------------------------------------------------

type BleResources = HostResources<DefaultPacketPool, 1, 2>;

#[embassy_executor::task]
async fn run_echo(sdc: SoftdeviceController<'static>) {
    static RESOURCES: StaticCell<BleResources> = StaticCell::new();
    let resources = RESOURCES.init(BleResources::new());

    // Fixed seed is fine — we do not initiate pairing so the PRNG is unused.
    let mut prng = ChaCha20Rng::from_seed([0u8; 32]);

    let stack = trouble_host::new(sdc, resources)
        .set_random_address(Address::random(device_id::get_ble_addr()))
        .set_random_generator_seed(&mut prng);

    // DisplayOnly: trouble-host generates a 6-digit passkey and fires PassKeyDisplay.
    // We log it to RTT.  The central (Linux/phone) asks the user to type the value.
    // After the first pairing the bond is cached — reconnections skip the passkey.
    stack.set_io_capabilities(IoCapabilities::DisplayOnly);

    let Host {
        mut peripheral,
        mut runner,
        ..
    } = stack.build();

    // Short name that fits in the 31-byte advertising payload without scan data.
    let name = b"BLE Echo";

    let mut adv_buf = [0u8; 31];
    let adv_len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(name),
        ],
        &mut adv_buf,
    )
    .unwrap();

    // NUS service UUID in scan response so NUS-aware tools can auto-detect.
    let mut scan_buf = [0u8; 31];
    let scan_len = AdStructure::encode_slice(
        &[AdStructure::ServiceUuids128(&[[
            0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0, 0x93, 0xf3, 0xa3, 0xb5, 0x01, 0x00,
            0x40, 0x6e,
        ]])],
        &mut scan_buf,
    )
    .unwrap();

    // Safety: name is valid ASCII, so it is valid UTF-8.
    let name_str = unsafe { core::str::from_utf8_unchecked(name) };
    let server = NusServer::new_default(name_str).unwrap();

    embassy_futures::join::join3(
        async {
            loop {
                if runner.run().await.is_err() {}
            }
        },
        // Feed the watchdog started by the bootloader (channel 0, 5-second timeout).
        // Without this the chip resets 5 s after boot and the bootloader rolls back.
        async {
            loop {
                embassy_nrf::pac::WDT
                    .rr(0)
                    .write(|w| w.set_rr(embassy_nrf::pac::wdt::vals::Rr::RELOAD));
                embassy_time::Timer::after_secs(1).await;
            }
        },
        async {
            loop {
                defmt::info!("Echo: advertising…");

                let advertiser = match peripheral
                    .advertise(
                        &Default::default(),
                        Advertisement::ConnectableScannableUndirected {
                            adv_data: &adv_buf[..adv_len],
                            scan_data: &scan_buf[..scan_len],
                        },
                    )
                    .await
                {
                    Ok(a) => a,
                    Err(e) => {
                        defmt::warn!("Echo: advertise error: {:?}", defmt::Debug2Format(&e));
                        embassy_time::Timer::after_millis(500).await;
                        continue;
                    }
                };

                let conn = match advertiser.accept().await {
                    Ok(c) => c,
                    Err(e) => {
                        defmt::warn!("Echo: accept error: {:?}", defmt::Debug2Format(&e));
                        continue;
                    }
                };

                defmt::info!("Echo: connected");

                // Enable bonding so the LTK is stored after pairing and reconnections
                // skip the passkey step.
                if let Err(e) = conn.set_bondable(true) {
                    defmt::warn!("Echo: set_bondable failed: {:?}", defmt::Debug2Format(&e));
                }

                let gatt_conn = match conn.with_attribute_server(&server.server) {
                    Ok(c) => c,
                    Err(e) => {
                        defmt::warn!("Echo: GATT setup error: {:?}", defmt::Debug2Format(&e));
                        continue;
                    }
                };

                loop {
                    match gatt_conn.next().await {
                        GattConnectionEvent::Disconnected { reason } => {
                            defmt::info!("Echo: disconnected ({:?})", defmt::Debug2Format(&reason));
                            break;
                        }
                        GattConnectionEvent::PassKeyDisplay(key) => {
                            defmt::info!(
                                "Echo: pairing passkey: {:06} — enter this on your device",
                                key.value()
                            );
                        }
                        GattConnectionEvent::PairingComplete {
                            bond,
                            security_level,
                        } => {
                            defmt::info!(
                                "Echo: pairing complete (level {:?})",
                                defmt::Debug2Format(&security_level)
                            );
                            let _ = bond; // bonds not persisted in test binary
                        }
                        GattConnectionEvent::PairingFailed(e) => {
                            defmt::warn!("Echo: pairing failed: {:?}", defmt::Debug2Format(&e));
                        }
                        GattConnectionEvent::Gatt {
                            event: GattEvent::Write(write),
                        } => {
                            if write.handle() == server.nus.rx.handle {
                                // Require authenticated encryption before echoing.
                                let sec = gatt_conn
                                    .raw()
                                    .security_level()
                                    .unwrap_or(SecurityLevel::NoEncryption);
                                if !sec.authenticated() {
                                    defmt::warn!("Echo: unauthenticated write — rejecting");
                                    if let Ok(reply) =
                                        write.reject(AttErrorCode::INSUFFICIENT_AUTHENTICATION)
                                    {
                                        reply.send().await;
                                    }
                                    continue;
                                }

                                let data = write.data();
                                defmt::info!("Echo: RX {} B: {=[u8]:a}", data.len(), data);

                                // Build echo payload before consuming `write` via accept().
                                let mut echo: heapless::Vec<u8, 244> = heapless::Vec::new();
                                let _ = echo.extend_from_slice(data);

                                // Acknowledge the write.
                                match write.accept() {
                                    Ok(reply) => reply.send().await,
                                    Err(e) => defmt::warn!(
                                        "Echo: write.accept() failed: {:?}",
                                        defmt::Debug2Format(&e)
                                    ),
                                }

                                // Echo back on TX.
                                match embassy_time::with_timeout(
                                    embassy_time::Duration::from_millis(2000),
                                    server.nus.tx.notify(&gatt_conn, &echo),
                                )
                                .await
                                {
                                    Ok(Ok(())) => {}
                                    Ok(Err(e)) => defmt::warn!(
                                        "Echo: TX notify failed: {:?}",
                                        defmt::Debug2Format(&e)
                                    ),
                                    Err(_) => defmt::warn!("Echo: TX notify timed out"),
                                }
                            } else {
                                // Accept any other writes (e.g. CCCD subscriptions).
                                if let Ok(reply) = write.accept() {
                                    reply.send().await;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                embassy_time::Timer::after_millis(200).await;
            }
        },
    )
    .await;
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);

    // Keep HFCLK active during sleep — matches debugger-attached behaviour.
    embassy_nrf::pac::POWER.tasks_constlat().write_value(1);

    device_id::init();
    defmt::info!("BLE echo test starting");

    // Buck-boost converter: low-power mode.
    let _ps_sync = Output::new(board!(p, ps_sync), Level::Low, OutputDrive::Standard);

    // Mark this firmware as successfully booted so the bootloader won't roll back
    // to the previous image on the next WDT reset.
    {
        let flash = Mutex::<NoopRawMutex, _>::new(core::cell::RefCell::new(Nvmc::new(p.NVMC)));
        let fw_config = FirmwareUpdaterConfig::from_linkerfile_blocking(&flash, &flash);
        let mut aligned = AlignedBuffer([0u8; 4]);
        let mut updater = BlockingFirmwareUpdater::new(fw_config, &mut aligned.0);
        let _ = updater.mark_booted();
        defmt::info!("Firmware marked as booted");
    }

    static SDC_MEM: StaticCell<nrf_sdc::Mem<{ hello_graphics::fw::ble::SDC_MEM_SIZE }>> =
        StaticCell::new();
    let sdc = init_ble(
        &spawner,
        p.RTC0,
        p.TIMER0,
        p.TEMP,
        p.PPI_CH19,
        p.PPI_CH30,
        p.PPI_CH31,
        p.PPI_CH17,
        p.PPI_CH18,
        p.PPI_CH20,
        p.PPI_CH21,
        p.PPI_CH22,
        p.PPI_CH23,
        p.PPI_CH24,
        p.PPI_CH25,
        p.PPI_CH26,
        p.PPI_CH27,
        p.PPI_CH28,
        p.PPI_CH29,
        p.RNG,
        SDC_MEM.init(nrf_sdc::Mem::new()),
    );

    spawner.must_spawn(run_echo(sdc));
}
