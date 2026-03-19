//! External QSPI flash driver with TicKV-backed BLE bond storage.
//!
//! Architecture:
//!   - `flash_task` owns the QSPI peripheral and the TicKV instance.
//!   - All flash access is serialized through that task via `BOND_CMD_CHANNEL`.
//!   - On startup the task populates `INITIAL_BONDS` (an OnceLock); the BLE task
//!     spins on that lock before starting the BLE runner.

use core::cell::UnsafeCell;

use embassy_nrf::{Peri, bind_interrupts, peripherals, qspi};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::once_lock::OnceLock;
use heapless::Vec;
use static_cell::StaticCell;
use tickv::{ErrorCode, FlashController, TicKV};
use trouble_host::prelude::{BdAddr, SecurityLevel};
use trouble_host::{BondInformation, Identity, IdentityResolvingKey, LongTermKey};

// ---------------------------------------------------------------------------
// Flash layout
// ---------------------------------------------------------------------------

/// TicKV region size = one erase sector on the QSPI flash (4 KiB).
const REGION_SIZE: usize = 4096;
/// Number of regions dedicated to bond storage (32 KiB total).
const NUM_REGIONS: usize = 8;
/// Byte offset in QSPI address space where bond storage begins.
const FLASH_BASE: u32 = 0;

const MAX_BONDS: usize = 4;

// ---------------------------------------------------------------------------
// Interrupt binding (QSPI)
// ---------------------------------------------------------------------------

bind_interrupts!(pub struct QspiIrqs {
    QSPI => qspi::InterruptHandler<peripherals::QSPI>;
});

// ---------------------------------------------------------------------------
// Deterministic FNV-1a 64-bit hash (no external dep, const-capable)
// ---------------------------------------------------------------------------

pub const fn fnv1a(data: &[u8]) -> u64 {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    let mut h = OFFSET;
    let mut i = 0;
    while i < data.len() {
        h ^= data[i] as u64;
        h = h.wrapping_mul(PRIME);
        i += 1;
    }
    h
}

/// TicKV "main key" that seeds the hash table.
const MAIN_KEY: u64 = fnv1a(b"cyberaegg_bonds_v1");
/// Well-known key that stores the list of bonded peer addresses.
const INDEX_KEY: u64 = fnv1a(b"bond_index");

fn bond_key(addr: &[u8; 6]) -> u64 {
    fnv1a(addr)
}

// ---------------------------------------------------------------------------
// QSPI flash controller for TicKV
// ---------------------------------------------------------------------------

/// 4-byte-aligned buffer for QSPI DMA reads/writes.
#[repr(C, align(4))]
pub struct AlignedBuf(pub [u8; REGION_SIZE]);

/// Aligned 256-byte staging area used when TicKV passes us an unaligned write slice.
struct WriteStagingBuf(UnsafeCell<[u32; 64]>); // 64 × 4 = 256 bytes, 4-byte aligned
unsafe impl Sync for WriteStagingBuf {}

static WRITE_STAGING: WriteStagingBuf = WriteStagingBuf(UnsafeCell::new([0u32; 64]));

pub struct QspiFlashController {
    /// Wrapped in UnsafeCell because FlashController takes &self, not &mut self.
    /// Safety: this type is only ever accessed from `flash_task` (single task).
    qspi: UnsafeCell<qspi::Qspi<'static>>,
}

// Safety: used exclusively from one task.
unsafe impl Send for QspiFlashController {}
unsafe impl Sync for QspiFlashController {}

impl QspiFlashController {
    fn qspi_mut(&self) -> &mut qspi::Qspi<'static> {
        // Safety: single-task access guaranteed by flash_task ownership.
        unsafe { &mut *self.qspi.get() }
    }
}

impl FlashController<REGION_SIZE> for QspiFlashController {
    fn read_region(
        &self,
        region_number: usize,
        buf: &mut [u8; REGION_SIZE],
    ) -> Result<(), ErrorCode> {
        let addr = FLASH_BASE + (region_number * REGION_SIZE) as u32;
        self.qspi_mut()
            .blocking_read(addr, buf)
            .map_err(|_| ErrorCode::ReadFail)
    }

    fn write(&self, address: usize, buf: &[u8]) -> Result<(), ErrorCode> {
        // nRF52840 QSPI DMA requires the write address, buffer pointer, and length to all
        // be 4-byte aligned. TicKV objects are 11 + value + 4 bytes and are not guaranteed
        // to satisfy these constraints, so we always go through the staging buffer.
        //
        // Strategy:
        //   1. Round address DOWN to 4-byte boundary; compute prefix length (0–3 bytes).
        //   2. If prefix > 0, read those bytes from flash — they belong to the previous
        //      TicKV object and must not be altered (NOR flash can't flip 0→1 without erase).
        //   3. Copy `buf` after the prefix; pad the trailer to the next 4-byte boundary
        //      with 0xFF (the erased-flash value, so TicKV sees "empty" for any padding).
        //   4. Issue a single aligned write of the padded staging slice.
        let addr = FLASH_BASE + address as u32;
        let aligned_addr = addr & !3;
        let prefix_len = (addr - aligned_addr) as usize;
        let total = prefix_len + buf.len();
        let padded_len = (total + 3) & !3;

        // Safety: WRITE_STAGING is only accessed from flash_task (single task).
        let staging_u32 = unsafe { &mut *WRITE_STAGING.0.get() };
        let staging = unsafe {
            core::slice::from_raw_parts_mut(staging_u32.as_mut_ptr() as *mut u8, 256)
        };
        assert!(padded_len <= 256, "TicKV write > 256 bytes");

        if prefix_len > 0 {
            // Read the 4-byte word that straddles the alignment boundary so we can
            // reproduce the already-written bytes faithfully in the write below.
            self.qspi_mut()
                .blocking_read(aligned_addr, &mut staging[..4])
                .map_err(|_| ErrorCode::WriteFail)?;
        }

        staging[prefix_len..total].copy_from_slice(buf);
        staging[total..padded_len].fill(0xFF);

        self.qspi_mut()
            .blocking_write(aligned_addr, &staging[..padded_len])
            .map_err(|_| ErrorCode::WriteFail)
    }

    fn erase_region(&self, region_number: usize) -> Result<(), ErrorCode> {
        let addr = FLASH_BASE + (region_number * REGION_SIZE) as u32;
        self.qspi_mut()
            .blocking_erase(addr)
            .map_err(|_| ErrorCode::EraseFail)
    }
}

// ---------------------------------------------------------------------------
// BondInformation serialization (fixed layout, 42 bytes)
// ---------------------------------------------------------------------------
//
//  [0..16]  LTK (u128 little-endian)
//  [16..22] bd_addr (6 bytes)
//  [22]     irk_present (0 or 1)
//  [23..39] IRK (u128 little-endian, zeroed if not present)
//  [39]     is_bonded (0 or 1)
//  [40]     security_level byte (see below)
//  [41]     reserved
//
// security_level encoding:
//   0 = Mode1(NoSecurityNeeded)   3 = Mode1(Lesc)
//   1 = Mode1(Unauthenticated)    other = unknown (treated as 0 on load)
//   2 = Mode1(Authenticated)
//
const BOND_SIZE: usize = 42;

fn security_level_to_u8(sl: &SecurityLevel) -> u8 {
    match sl {
        SecurityLevel::NoEncryption => 0,
        SecurityLevel::Encrypted => 1,
        SecurityLevel::EncryptedAuthenticated => 2,
    }
}

fn security_level_from_u8(b: u8) -> SecurityLevel {
    match b {
        1 => SecurityLevel::Encrypted,
        2 => SecurityLevel::EncryptedAuthenticated,
        _ => SecurityLevel::NoEncryption,
    }
}

fn serialize_bond(info: &BondInformation) -> [u8; BOND_SIZE] {
    let mut buf = [0u8; BOND_SIZE];
    buf[0..16].copy_from_slice(&info.ltk.0.to_le_bytes());
    let addr = info.identity.bd_addr.into_inner();
    buf[16..22].copy_from_slice(&addr);
    if let Some(irk) = info.identity.irk {
        buf[22] = 1;
        buf[23..39].copy_from_slice(&irk.0.to_le_bytes());
    }
    buf[39] = info.is_bonded as u8;
    buf[40] = security_level_to_u8(&info.security_level);
    buf
}

fn deserialize_bond(buf: &[u8; BOND_SIZE]) -> BondInformation {
    let ltk = LongTermKey(u128::from_le_bytes(buf[0..16].try_into().unwrap()));
    let mut addr = [0u8; 6];
    addr.copy_from_slice(&buf[16..22]);
    let irk = if buf[22] != 0 {
        let key = u128::from_le_bytes(buf[23..39].try_into().unwrap());
        Some(IdentityResolvingKey(key))
    } else {
        None
    };
    let identity = Identity {
        bd_addr: BdAddr::new(addr),
        irk,
    };
    BondInformation::new(identity, ltk, security_level_from_u8(buf[40]), buf[39] != 0)
}

// ---------------------------------------------------------------------------
// BondStore — wraps TicKV with a bond-index for enumeration
// ---------------------------------------------------------------------------

pub struct BondStore<'a> {
    tickv: TicKV<'a, QspiFlashController, REGION_SIZE>,
}

impl<'a> BondStore<'a> {
    pub fn new(ctrl: QspiFlashController, buf: &'a mut AlignedBuf) -> Self {
        let tickv = TicKV::new(ctrl, &mut buf.0, NUM_REGIONS * REGION_SIZE);
        let store = Self { tickv };
        // Initialise TicKV; KeyNotFound on a blank flash is expected on first boot.
        // UnsupportedVersion means leftover data from an incompatible firmware — erase and retry.
        if let Err(ErrorCode::UnsupportedVersion) = store.tickv.initialise(MAIN_KEY) {
            defmt::warn!("TicKV: incompatible data found, erasing bond store regions");
            for r in 0..NUM_REGIONS {
                let _ = store.tickv.controller.erase_region(r);
            }
            match store.tickv.initialise(MAIN_KEY) {
                Ok(_) | Err(ErrorCode::KeyNotFound) => {}
                Err(e) => defmt::warn!("TicKV re-init: {:?}", defmt::Debug2Format(&e)),
            }
        }
        store
    }

    /// Save a bond (or update an existing one).
    pub fn save(&mut self, info: &BondInformation) {
        let addr = info.identity.bd_addr.into_inner();
        let key = bond_key(&addr);
        let data = serialize_bond(info);
        // Invalidate any previous entry, then append the new one.
        let _ = self.tickv.invalidate_key(key);
        match self.tickv.append_key(key, &data) {
            Ok(_) => {}
            Err(e) => defmt::warn!("BondStore::save error: {:?}", defmt::Debug2Format(&e)),
        }
        self.update_index();
    }

    /// Remove a bond by peer address.
    pub fn remove(&mut self, addr: &[u8; 6]) {
        let _ = self.tickv.invalidate_key(bond_key(addr));
        self.update_index();
    }

    /// Load all stored bonds at startup.
    pub fn load_all(&mut self) -> Vec<BondInformation, MAX_BONDS> {
        let mut out = Vec::new();
        let mut index_buf = [0u8; MAX_BONDS * 6];
        let n = match self.tickv.get_key(INDEX_KEY, &mut index_buf) {
            Ok((_code, n)) => n,
            Err(ErrorCode::KeyNotFound) => return out,
            Err(e) => {
                defmt::warn!("BondStore: index read error: {:?}", defmt::Debug2Format(&e));
                return out;
            }
        };
        for chunk in index_buf[..n].chunks_exact(6) {
            let mut addr = [0u8; 6];
            addr.copy_from_slice(chunk);
            let mut buf = [0u8; BOND_SIZE];
            match self.tickv.get_key(bond_key(&addr), &mut buf) {
                Ok(_) => {
                    let _ = out.push(deserialize_bond(&buf));
                }
                Err(e) => {
                    defmt::warn!("BondStore: bond read error: {:?}", defmt::Debug2Format(&e));
                }
            }
        }
        out
    }

    /// Rebuild the index entry from scratch by scanning known bonds.
    fn update_index(&mut self) {
        // Re-read current index, collect valid bonds still present, rewrite.
        let mut addrs: Vec<[u8; 6], MAX_BONDS> = Vec::new();
        let mut index_buf = [0u8; MAX_BONDS * 6];
        if let Ok((_code, n)) = self.tickv.get_key(INDEX_KEY, &mut index_buf) {
            for chunk in index_buf[..n].chunks_exact(6) {
                let mut addr = [0u8; 6];
                addr.copy_from_slice(chunk);
                // Check the bond still exists.
                let mut tmp = [0u8; BOND_SIZE];
                if self.tickv.get_key(bond_key(&addr), &mut tmp).is_ok() {
                    let _ = addrs.push(addr);
                }
            }
        }
        // Write updated index.
        let mut flat = [0u8; MAX_BONDS * 6];
        for (i, addr) in addrs.iter().enumerate() {
            flat[i * 6..(i + 1) * 6].copy_from_slice(addr);
        }
        let _ = self.tickv.invalidate_key(INDEX_KEY);
        let _ = self.tickv.append_key(INDEX_KEY, &flat[..addrs.len() * 6]);
    }
}

// ---------------------------------------------------------------------------
// IPC: BLE task → flash task
// ---------------------------------------------------------------------------

pub enum BondCmd {
    Save(BondInformation),
    Remove([u8; 6]),
}

pub static BOND_CMD_CHANNEL: Channel<CriticalSectionRawMutex, BondCmd, 4> = Channel::new();

/// Populated by `flash_task` before advertising starts; BLE task waits on this.
pub static INITIAL_BONDS: OnceLock<Vec<BondInformation, MAX_BONDS>> = OnceLock::new();

// ---------------------------------------------------------------------------
// QSPI init helper
// ---------------------------------------------------------------------------

pub fn init_qspi<'d>(
    qspi_periph: Peri<'d, peripherals::QSPI>,
    irqs: QspiIrqs,
    sck: Peri<'d, peripherals::P0_21>,
    csn: Peri<'d, peripherals::P0_25>,
    io0: Peri<'d, peripherals::P0_20>,
    io1: Peri<'d, peripherals::P0_24>,
    io2: Peri<'d, peripherals::P0_22>,
    io3: Peri<'d, peripherals::P0_23>,
) -> Result<qspi::Qspi<'d>, [u8; 3]> {
    // ZD25WQ16CTIGT: 16 Mbit = 2 MiB. All boards use this chip.
    // Use single-SPI opcodes (FASTREAD/PP) — quad I/O (READ4IO/PP4IO) requires the
    // Quad Enable (QE) bit to be set in the chip's status register, which we don't
    // configure here. Single-SPI is sufficient for the infrequent bond storage operations.
    let mut cfg = qspi::Config::default();
    cfg.capacity = 2 * 1024 * 1024;
    cfg.read_opcode = qspi::ReadOpcode::FASTREAD;
    cfg.write_opcode = qspi::WriteOpcode::PP;
    let mut qspi = qspi::Qspi::new(
        qspi_periph,
        irqs,
        sck,
        csn,
        io0,
        io1,
        io2,
        io3,
        cfg,
    );
    // Read JEDEC ID (opcode 0x9F) to verify the chip is present and responding.
    // All-0xFF means no device on the bus; all-0x00 means a shorted/stuck bus.
    let mut jedec = [0u8; 3];
    let _ = qspi.blocking_custom_instruction(0x9F, &[], &mut jedec);
    if jedec == [0xFF; 3] || jedec == [0x00; 3] {
        return Err(jedec);
    }
    defmt::info!("QSPI flash JEDEC ID: {:02X} {:02X} {:02X}", jedec[0], jedec[1], jedec[2]);
    Ok(qspi)
}

// ---------------------------------------------------------------------------
// flash_task
// ---------------------------------------------------------------------------

#[embassy_executor::task]
pub async fn flash_task(qspi: qspi::Qspi<'static>) {
    static TICKV_BUF: StaticCell<AlignedBuf> = StaticCell::new();
    let buf = TICKV_BUF.init(AlignedBuf([0u8; REGION_SIZE]));
    let ctrl = QspiFlashController {
        qspi: UnsafeCell::new(qspi),
    };
    let mut store = BondStore::new(ctrl, buf);

    // Load persisted bonds and make them available to the BLE task.
    let bonds = store.load_all();
    defmt::info!("BondStore: loaded {} bond(s)", bonds.len());
    let _ = INITIAL_BONDS.init(bonds);

    // Service bond commands from the BLE task.
    let rx = BOND_CMD_CHANNEL.receiver();
    loop {
        match rx.receive().await {
            BondCmd::Save(info) => {
                defmt::info!("BondStore: saving bond for {:?}", info.identity.bd_addr);
                store.save(&info);
            }
            BondCmd::Remove(addr) => {
                defmt::info!("BondStore: removing bond");
                store.remove(&addr);
            }
        }
    }
}
