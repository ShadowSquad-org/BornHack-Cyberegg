//! Device identity: a two-byte ID from the nRF52840's factory-programmed FICR device address.
//!
//! Call `init()` once at startup; retrieve the ID from anywhere with `get()`.

use embassy_sync::once_lock::OnceLock;

static DEVICE_ID: OnceLock<[u8; 2]> = OnceLock::new();

/// Read the two-byte ID from FICR DEVICEADDR[0] and cache it.  Call once at startup.
pub fn init() {
    let lo = embassy_nrf::pac::FICR.deviceaddr(0).read();
    let b = lo.to_le_bytes();
    let _ = DEVICE_ID.init([b[0], b[1]]);
}

/// Return the cached two-byte device ID.  Panics if `init()` has not been called.
pub fn get() -> [u8; 2] {
    *DEVICE_ID.try_get().expect("device_id::init() not called")
}
