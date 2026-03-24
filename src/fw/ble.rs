//! BLE peripheral using TrouBLE (trouble-host) over nrf-sdc/nrf-mpsl.
//!
//! Exposes a Nordic UART Service (NUS) for MeshCore companion app connectivity.
//! Bonding keys are persisted to QSPI flash via `flash_task`; see flash.rs.

use core::sync::atomic::Ordering;

use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;

use meshcore_companion as companion;

use embassy_executor::Spawner;
use embassy_nrf::{Peri, bind_interrupts, mode::Blocking, peripherals, rng};
use nrf_mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, SoftdeviceController};
use static_cell::StaticCell;
use trouble_host::prelude::*;

use crate::fw::bonds::{BOND_CMD_CHANNEL, INITIAL_BONDS, BondCmd};

// ---------------------------------------------------------------------------
// Interrupt bindings for MPSL + RNG
// ---------------------------------------------------------------------------

bind_interrupts!(pub struct BleIrqs {
    EGU0_SWI0   => nrf_mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_mpsl::ClockInterruptHandler;
    RADIO       => nrf_mpsl::HighPrioInterruptHandler;
    TIMER0      => nrf_mpsl::HighPrioInterruptHandler;
    RTC0        => nrf_mpsl::HighPrioInterruptHandler;
    RNG         => rng::InterruptHandler<peripherals::RNG>;
});

// ---------------------------------------------------------------------------
// MPSL task
// ---------------------------------------------------------------------------

static MPSL: StaticCell<MultiprotocolServiceLayer<'static>> = StaticCell::new();

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

// ---------------------------------------------------------------------------
// MPSL + SDC initialisation
// ---------------------------------------------------------------------------

/// Initialise the Multiprotocol Service Layer and SoftDevice Controller.
///
/// Returns the SDC and a blocking RNG — keep both alive in main's scope.
/// Initialise MPSL + SDC.  Called from `#[embassy_executor::main]` where all
/// `Peri<'d, T>` tokens are `'static`, so the `'d: 'static` bound is satisfied.
pub fn init_ble(
    spawner: &Spawner,
    // MPSL
    rtc0:     Peri<'static, peripherals::RTC0>,
    timer0:   Peri<'static, peripherals::TIMER0>,
    temp:     Peri<'static, peripherals::TEMP>,
    ppi_ch19: Peri<'static, peripherals::PPI_CH19>,
    ppi_ch30: Peri<'static, peripherals::PPI_CH30>,
    ppi_ch31: Peri<'static, peripherals::PPI_CH31>,
    // SDC
    ppi_ch17: Peri<'static, peripherals::PPI_CH17>,
    ppi_ch18: Peri<'static, peripherals::PPI_CH18>,
    ppi_ch20: Peri<'static, peripherals::PPI_CH20>,
    ppi_ch21: Peri<'static, peripherals::PPI_CH21>,
    ppi_ch22: Peri<'static, peripherals::PPI_CH22>,
    ppi_ch23: Peri<'static, peripherals::PPI_CH23>,
    ppi_ch24: Peri<'static, peripherals::PPI_CH24>,
    ppi_ch25: Peri<'static, peripherals::PPI_CH25>,
    ppi_ch26: Peri<'static, peripherals::PPI_CH26>,
    ppi_ch27: Peri<'static, peripherals::PPI_CH27>,
    ppi_ch28: Peri<'static, peripherals::PPI_CH28>,
    ppi_ch29: Peri<'static, peripherals::PPI_CH29>,
    // RNG
    rng_periph: Peri<'static, peripherals::RNG>,
    sdc_mem: &'static mut sdc::Mem<4096>,
) -> SoftdeviceController<'static> {
    // 32 kHz crystal fitted on the board.
    let lfclk_cfg = nrf_mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: nrf_mpsl::raw::MPSL_CLOCK_LF_SRC_XTAL as u8,
        rc_ctiv: 0,
        rc_temp_ctiv: 0,
        accuracy_ppm: 20,
        skip_wait_lfclk_started: false,
    };

    let mpsl_p = nrf_mpsl::Peripherals::new(rtc0, timer0, temp, ppi_ch19, ppi_ch30, ppi_ch31);
    let mpsl = MPSL.init(
        nrf_mpsl::MultiprotocolServiceLayer::new(mpsl_p, BleIrqs, lfclk_cfg).unwrap(),
    );
    spawner.must_spawn(mpsl_task(mpsl));

    let sdc_p = sdc::Peripherals::new(
        ppi_ch17, ppi_ch18, ppi_ch20, ppi_ch21, ppi_ch22, ppi_ch23,
        ppi_ch24, ppi_ch25, ppi_ch26, ppi_ch27, ppi_ch28, ppi_ch29,
    );

    // nrf-sdc 0.4: build() takes `rng: &'static mut Rng` and stores a raw pointer to it
    // in a global for use by the SDC's random callback.  StaticCell gives us the 'static
    // storage; the peripheral token is already 'static so no unsafe is needed.
    static RNG_STORAGE: StaticCell<rng::Rng<'static, Blocking>> = StaticCell::new();
    let rng_ref = RNG_STORAGE.init(rng::Rng::new_blocking(rng_periph));

    // In nrf-sdc 0.4, support_adv/support_peripheral return Self directly (not Result).
    let sdc = sdc::Builder::new()
        .unwrap()
        .support_adv()
        .support_peripheral()
        .peripheral_count(1)
        .unwrap()
        .build(sdc_p, rng_ref, mpsl, sdc_mem)
        .unwrap();

    defmt::info!("BLE: MPSL + SDC initialised");
    sdc
}

// ---------------------------------------------------------------------------
// Nordic UART Service (NUS) GATT definition
// ---------------------------------------------------------------------------

/// NUS service UUID: 6E400001-B5A3-F393-E0A9-E50E24DCCA9E
#[gatt_service(uuid = "6e400001-b5a3-f393-e0a9-e50e24dcca9e")]
pub struct NusService {
    /// RX characteristic — phone writes frames to the badge.
    #[characteristic(
        uuid = "6e400002-b5a3-f393-e0a9-e50e24dcca9e",
        write,
        write_without_response
    )]
    pub rx: [u8; 20],

    /// TX characteristic — badge notifies frames to the phone.
    #[characteristic(uuid = "6e400003-b5a3-f393-e0a9-e50e24dcca9e", notify)]
    pub tx: [u8; 20],
}

#[gatt_server]
pub struct NusServer {
    pub nus: NusService,
}

// ---------------------------------------------------------------------------
// Companion protocol context + helpers
// ---------------------------------------------------------------------------

/// Device information snapshot passed to the companion protocol handler.
/// Filled in by `embassy.rs` at startup from the identity and radio config.
pub struct CompanionContext {
    /// Ed25519 public key (32 bytes).
    pub pub_key: [u8; 32],
    /// LoRa radio frequency in Hz (e.g. 869_618_000).
    pub frequency_hz: u32,
    /// LoRa radio bandwidth in Hz (e.g. 62_500).
    pub bandwidth_hz: u32,
    /// LoRa spreading factor value (e.g. 8 for SF8).
    pub spreading_factor: u8,
    /// LoRa coding rate: 1 = 4/5.
    pub coding_rate: u8,
    /// TX power in dBm.
    pub tx_power: i8,
}

/// Send `data` as one or more 20-byte NUS TX notifications, zero-padding the
/// last chunk.  The companion app reassembles multi-notification responses.
async fn notify_chunked<P: PacketPool>(
    server: &NusServer<'_>,
    conn: &GattConnection<'_, '_, P>,
    data: &[u8],
) {
    let mut offset = 0;
    let total_chunks = (data.len() + companion::CHUNK_SIZE - 1) / companion::CHUNK_SIZE;
    let mut chunk_idx = 0usize;
    while offset < data.len() {
        let end = (offset + companion::CHUNK_SIZE).min(data.len());
        let mut chunk = [0u8; companion::CHUNK_SIZE];
        chunk[..end - offset].copy_from_slice(&data[offset..end]);
        if let Err(e) = server.nus.tx.notify(conn, &chunk).await {
            defmt::warn!("companion: notify chunk {}/{} failed: {:?}", chunk_idx + 1, total_chunks, defmt::Debug2Format(&e));
        }
        offset += companion::CHUNK_SIZE;
        chunk_idx += 1;
    }
}

// ---------------------------------------------------------------------------
// BLE peripheral runner
// ---------------------------------------------------------------------------

type BleResources = HostResources<DefaultPacketPool, 1, 2>;

#[embassy_executor::task]
pub async fn run_ble_peripheral(sdc: SoftdeviceController<'static>, ctx: CompanionContext, prng_seed: [u8; 32]) {
    static RESOURCES: StaticCell<BleResources> = StaticCell::new();
    let resources = RESOURCES.init(BleResources::new());

    // Seed the security manager PRNG from TRNG entropy collected at startup
    // (before the RNG peripheral was consumed by the SDC).
    let mut prng = ChaCha20Rng::from_seed(prng_seed);

    let stack = trouble_host::new(sdc, resources)
        .set_random_address(Address::random(crate::fw::device_id::get_ble_addr()))
        .set_random_generator_seed(&mut prng);

    // DisplayOnly: badge shows a 6-digit passkey on screen; the phone user enters it.
    // This matches MeshCore's setIOCaps(true, false, false) and enables MITM protection.
    stack.set_io_capabilities(IoCapabilities::DisplayOnly);

    // Restore bonds loaded from flash by flash_task.
    // Spin briefly if flash_task hasn't populated INITIAL_BONDS yet.
    loop {
        if let Some(bonds) = INITIAL_BONDS.try_get() {
            for (i, bond) in bonds.iter().enumerate() {
                let addr = bond.identity.bd_addr.into_inner();
                match stack.add_bond_information(bond.clone()) {
                    Ok(()) => defmt::info!(
                        "BLE: restored bond[{}] addr={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                        i, addr[0], addr[1], addr[2], addr[3], addr[4], addr[5]
                    ),
                    Err(e) => defmt::warn!(
                        "BLE: failed to restore bond[{}]: {:?}",
                        i, defmt::Debug2Format(&e)
                    ),
                }
            }
            defmt::info!("BLE: restored {} bond(s) from flash", bonds.len());
            break;
        }
        embassy_time::Timer::after_millis(1).await;
    }

    let Host { mut peripheral, mut runner, .. } = stack.build();

    let bond_tx = BOND_CMD_CHANNEL.sender();

    // Run the HCI runner in parallel with the advertising loop.
    embassy_futures::join::join(
        async { loop { if runner.run().await.is_err() {} } },
        nus_peripheral_loop(&mut peripheral, bond_tx, &ctx),
    )
    .await;
}

async fn nus_peripheral_loop<C>(
    peripheral: &mut Peripheral<'_, C, DefaultPacketPool>,
    bond_tx: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, BondCmd, 4>,
    ctx: &CompanionContext,
) where
    C: Controller,
{
    // Build the device name: "Cyber Ægg XXYY" where XXYY is the two-byte device ID in hex.
    // Flags (3 B) + name (16 B) = 19 B — fits within the 31-byte adv packet limit.
    // The 128-bit NUS UUID (18 B) goes into scan_data so the total doesn't overflow.
    // "Cyber Ægg XXYY" — Æ (U+00C6) is 0xC3 0x86 in UTF-8, total 15 bytes.
    let id = crate::fw::device_id::get_bytes();
    let name: [u8; 15] = [
        b'C', b'y', b'b', b'e', b'r', b' ',
        0xC3, 0x86, b'g', b'g', b' ',
        id[0], id[1], id[2], id[3],
    ];
    // Safety: all bytes are valid UTF-8 (ASCII + the two-byte Æ sequence above).
    let name_str = unsafe { core::str::from_utf8_unchecked(&name) };

    let mut adv_buf = [0u8; 31];
    let adv_len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(&name),
        ],
        &mut adv_buf,
    ).unwrap();

    let mut scan_buf = [0u8; 31];
    let scan_len = AdStructure::encode_slice(
        &[AdStructure::ServiceUuids128(&[
            [0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0,
             0x93, 0xf3, 0xa3, 0xb5, 0x01, 0x00, 0x40, 0x6e],
        ])],
        &mut scan_buf,
    ).unwrap();

    let server = NusServer::new_default(name_str).unwrap();

    loop {
        defmt::debug!("BLE: advertising…");

        let advertiser = match peripheral
            .advertise(
                &Default::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data:  &adv_buf[..adv_len],
                    scan_data: &scan_buf[..scan_len],
                },
            )
            .await
        {
            Ok(a) => a,
            Err(e) => {
                defmt::warn!("BLE: advertise error: {:?}", defmt::Debug2Format(&e));
                embassy_time::Timer::after_millis(500).await;
                continue;
            }
        };

        let conn = match advertiser.accept().await {
            Ok(c) => c,
            Err(e) => {
                defmt::warn!("BLE: accept error: {:?}", defmt::Debug2Format(&e));
                continue;
            }
        };

        defmt::info!("BLE: connected");

        let gatt_conn = match conn.with_attribute_server(&server.server) {
            Ok(c) => c,
            Err(e) => {
                defmt::warn!("BLE: gatt setup error: {:?}", defmt::Debug2Format(&e));
                continue;
            }
        };

        // Gate: only process companion commands after pairing/encryption is confirmed.
        // Set when PairingComplete fires (new pairing or bonded reconnect).
        let mut authenticated = false;

        loop {
            match gatt_conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    defmt::info!("BLE: disconnected (reason {:?})", defmt::Debug2Format(&reason));
                    crate::BLE_PASSKEY.store(u32::MAX, Ordering::Relaxed);
                    crate::BLE_PAIRING_SIGNAL.signal(());
                    break;
                }
                GattConnectionEvent::PassKeyDisplay(key) => {
                    defmt::info!("BLE: pairing passkey: {:06}", key.value());
                    crate::BLE_PASSKEY.store(key.value(), Ordering::Relaxed);
                    crate::BLE_PAIRING_SIGNAL.signal(());
                }
                GattConnectionEvent::PairingComplete { bond, security_level } => {
                    defmt::info!("BLE: pairing complete (level {:?})", defmt::Debug2Format(&security_level));
                    crate::BLE_PASSKEY.store(u32::MAX, Ordering::Relaxed);
                    crate::BLE_PAIRING_SIGNAL.signal(());
                    authenticated = true;
                    if let Some(info) = bond {
                        defmt::info!("BLE: new bond — persisting");
                        let _ = bond_tx.try_send(BondCmd::Save(info));
                    } else {
                        defmt::info!("BLE: bonded reconnect — using stored LTK");
                    }
                }
                GattConnectionEvent::PairingFailed(e) => {
                    defmt::warn!("BLE: pairing failed: {:?}", defmt::Debug2Format(&e));
                    crate::BLE_PASSKEY.store(u32::MAX, Ordering::Relaxed);
                    crate::BLE_PAIRING_SIGNAL.signal(());
                    // authenticated stays false — GATT writes will be rejected.
                }
                GattConnectionEvent::Gatt { event: GattEvent::Write(write) } => {
                    if write.handle() == server.nus.rx.handle {
                        if !authenticated {
                            defmt::warn!("companion: write before auth — rejecting");
                            if let Ok(reply) = write.accept() { reply.send().await; }
                            continue;
                        }
                        let data = write.data();
                        defmt::info!("companion RX {} bytes: {:02x}", data.len(), data);

                        // Declare before the match so its lifetime covers `response`.
                        let device_name = crate::fw::device_id::get_bytes();
                        let response = match companion::cmd::parse(data) {
                            Err(_) => {
                                defmt::warn!("companion: empty write");
                                companion::Response::Error
                            }

                            Ok(companion::cmd::Command::AppStart) => {
                                defmt::info!("companion: APP_START → SELF_INFO");
                                companion::Response::SelfInfo(companion::SelfInfo {
                                    adv_type: 1,       // ChatNode
                                    tx_power: ctx.tx_power,
                                    max_tx_power: 22,
                                    pub_key: &ctx.pub_key,
                                    lat: 0,
                                    lon: 0,
                                    multi_acks: 0,
                                    adv_location_policy: 0,
                                    telemetry_mode: 0,
                                    manual_add_contacts: 0,
                                    frequency_hz: ctx.frequency_hz,
                                    bandwidth_hz: ctx.bandwidth_hz,
                                    spreading_factor: ctx.spreading_factor,
                                    coding_rate: ctx.coding_rate,
                                    name: &device_name,
                                })
                            }

                            Ok(companion::cmd::Command::DeviceQuery) => {
                                defmt::info!("companion: DEVICE_QUERY → DEVICE_INFO");
                                companion::Response::DeviceInfo(companion::DeviceInfo {
                                    fw_version: 3,
                                    max_contacts_raw: 10,  // 20 contacts
                                    max_channels: 8,
                                    ble_pin: {
                                        let v = crate::BLE_PASSKEY.load(Ordering::Relaxed);
                                        if v == u32::MAX { 0 } else { v }
                                    },
                                    fw_build: b"dev",
                                    model: b"BornHack Cyber\xC3\x86gg",
                                    version: b"0.1.0",
                                    client_repeat: false,
                                    path_hash_mode: 0,
                                })
                            }

                            Ok(companion::cmd::Command::GetBattery) => {
                                defmt::info!("companion: GET_BATT → BATTERY");
                                let pct = crate::fw::battery::read_pct() as u16;
                                companion::Response::Battery {
                                    mv: 3000 + pct * 12,
                                    used_kb: 0,
                                    total_kb: 8192,
                                }
                            }

                            Ok(companion::cmd::Command::SyncNextMessage)
                            | Ok(companion::cmd::Command::GetContacts)
                            | Ok(companion::cmd::Command::GetChannel(_)) => {
                                defmt::info!("companion: msg/contact/channel query → NO_MORE_MSGS");
                                companion::Response::NoMoreMsgs
                            }

                            Ok(companion::cmd::Command::SetDeviceTime(ts)) => {
                                defmt::info!("companion: SET_DEVICE_TIME ts={=u32} → OK", ts);
                                companion::Response::Ok
                            }

                            Ok(companion::cmd::Command::SetChannel { .. }) => {
                                defmt::info!("companion: SET_CHANNEL → OK (not stored)");
                                companion::Response::Ok
                            }

                            Ok(companion::cmd::Command::Unknown(b)) => {
                                defmt::warn!("companion: unknown command 0x{:02X} → ERROR", b);
                                companion::Response::Error
                            }
                        };

                        let mut resp_buf = [0u8; companion::MAX_RESPONSE_LEN];
                        let resp_len = companion::encode(&response, &mut resp_buf);
                        defmt::info!("companion TX {} bytes: {:02x}", resp_len, &resp_buf[..resp_len]);

                        // Acknowledge the write BEFORE sending notifications.
                        // The phone waits for the ATT Write Response before it
                        // will process any incoming notifications; reversing the
                        // order causes the "Connecting…" stall.
                        if let Ok(reply) = write.accept() {
                            reply.send().await;
                        }
                        notify_chunked(&server, &gatt_conn, &resp_buf[..resp_len]).await;
                    } else if let Ok(reply) = write.accept() {
                        reply.send().await;
                    }
                }
                _ => {}
            }
        }

        // Give the HCI runner time to fully process the disconnection before
        // the outer loop tries to start advertising again.  Without this
        // delay the advertiser immediately gets "Connection Rejected due to
        // Limited Resources" because the controller slot isn't freed yet.
        embassy_time::Timer::after_millis(200).await;
    }
}
