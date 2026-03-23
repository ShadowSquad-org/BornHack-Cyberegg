use core::cell::RefCell;

use embassy_nrf::{Peri, gpio::AnyPin, peripherals};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Timer;

use super::device_identity::DeviceIdentity;
use super::health::SYSTEM_HEALTH;
use super::sx1262::{MeshCoreConfig, SimpleLoRa};
use crate::{health_err, update_health};
use meshcore::channel::KnownChannel;
use meshcore::contacts::Contacts;
use meshcore::dedup::{MsgHashRing, msg_hash};

static CONTACTS: Mutex<CriticalSectionRawMutex, RefCell<Contacts<20>>> =
    Mutex::new(RefCell::new(Contacts::new()));

static MSG_SEEN: Mutex<CriticalSectionRawMutex, RefCell<MsgHashRing<50>>> =
    Mutex::new(RefCell::new(MsgHashRing::new()));

// ---------------------------------------------------------------------------
// MeshCore listener task
// ---------------------------------------------------------------------------

/// Listen for MeshCore packets on the SX1262 and store decoded messages.
///
/// Configures the SX1262 using [`MeshCoreConfig::UK_NARROW_BAND`] and enters
/// a continuous receive loop.  Every received packet is parsed with the
/// `meshcore` vendor crate.  Group-text messages (`GrpTxt`) are decoded,
/// deduplicated, and stored in `LAST_LORA_MSG`; node advertisements are
/// logged; all other types are logged as raw hex.
pub async fn run_meshcore_listener<'a>(
    spi: Peri<'a, peripherals::SPI2>,
    sck_pin: Peri<'a, AnyPin>,
    mosi_pin: Peri<'a, AnyPin>,
    miso_pin: Peri<'a, AnyPin>,
    nrst_pin: Peri<'a, AnyPin>,
    nss_pin: Peri<'a, AnyPin>,
    busy_pin: Peri<'a, AnyPin>,
    dio1_pin: Peri<'a, AnyPin>,
    ant_pin: Peri<'a, AnyPin>,
    identity: &DeviceIdentity,
) -> ! {
    update_health!(|h| h.lora.set_ok("Ok when started."));

    let config = &MeshCoreConfig::UK_NARROW_BAND;

    let mut lora = match SimpleLoRa::new(
        spi, sck_pin, mosi_pin, miso_pin, nrst_pin, nss_pin, busy_pin, dio1_pin, ant_pin, config,
    ) {
        Ok(l) => {
            SYSTEM_HEALTH.lock(|cell| {
                cell.borrow_mut().lora.set_ok("SX1262 init OK");
            });
            l
        }
        Err(e) => {
            health_err!(lora, "LoRa init failed");
            defmt::error!("LoRa init failed: {:?}", e);
            loop {
                Timer::after_millis(60_000).await;
            }
        }
    };

    if !lora.ensure_rx().await {
        defmt::error!(
            "SX1262 failed to enter RX mode after 500ms — check crystal/wiring"
        );
    }

    defmt::info!(
        "MeshCore listener ready — freq={=u32}Hz BW=62.5kHz SF=8 CR=4/5 sync={=u16:#06x} preamble={=u16}",
        config.frequency_hz,
        config.sync_word,
        config.preamble_len,
    );
    defmt::info!("MeshCore identity pub_key: {=[u8]:02x}", &identity.pub_key[..]);

    let channels = [
        KnownChannel::public(),
        KnownChannel::hashtag("#test"),
        KnownChannel::hashtag("#prut"),
        KnownChannel::hashtag("#gezellig"),
        KnownChannel::hashtag("#leiden"),
    ];

    let mut raw = [0u8; 255];

    loop {
        match lora.receive_packet(&mut raw).await {
            Ok(None) => { /* timeout or CRC error — already re-armed */ }

            Ok(Some((len, rssi))) => {
                let frame = &raw[..len];

                match meshcore::packet::deserialize(frame) {
                    Err(_) => {
                        defmt::info!(
                            "MeshCore [raw {=usize}B {=i16}dBm]: {=[u8]}",
                            len,
                            rssi,
                            frame
                        );
                    }

                    Ok(msg) => {
                        update_health!(|h| h.lora.set_ok("Packet received."));
                        use meshcore::packet::PayloadType;
                        match msg.payload_type {
                            PayloadType::GrpTxt => log_grp_txt(&msg.payload, rssi, &channels),
                            PayloadType::TxtMsg => log_txt_msg(&msg.payload, rssi, identity),
                            PayloadType::Advert => log_advert(&msg.payload, rssi),
                            PayloadType::Ack => defmt::info!("MeshCore Ack [{=i16}dBm]", rssi),
                            other => {
                                defmt::info!(
                                    "MeshCore type={=u8} [{=usize}B {=i16}dBm]: {=[u8]:x}",
                                    other.to_u8(),
                                    len,
                                    rssi,
                                    frame
                                );
                            }
                        }
                    }
                }
            }

            Err(e) => {
                defmt::error!("LoRa RX error: {:?}", e);
                health_err!(lora, "LoRa RX error");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-type handlers
// ---------------------------------------------------------------------------

fn log_grp_txt(payload: &[u8], rssi: i16, channels: &[KnownChannel]) {
    use meshcore::payload::grp_txt;

    let grp = match grp_txt::deserialize(payload) {
        Ok(g) => g,
        Err(_) => {
            defmt::warn!("GrpTxt: failed to parse payload");
            return;
        }
    };

    let ch = match channels.iter().find(|c| c.hash == grp.channel_hash) {
        Some(c) => c,
        None => {
            defmt::info!(
                "MeshCore GrpTxt [channel={=u8} {=i16}dBm] (unknown channel): {=[u8]}",
                grp.channel_hash,
                rssi,
                &grp.data[..]
            );
            return;
        }
    };

    if grp_txt::verify_mac(&ch.key, &grp).is_err() {
        defmt::warn!(
            "MeshCore GrpTxt [channel={=u8}] MAC mismatch on channel {=str}",
            grp.channel_hash,
            ch.name
        );
        return;
    }

    match grp_txt::decrypt(&ch.key, &grp) {
        Ok(dec) => {
            let text = core::str::from_utf8(&dec.text).unwrap_or("<invalid utf-8>");

            // Deduplicate: hash channel_hash + decrypted text + timestamp
            // (stable across mesh hops — repeated relays don't change this data).
            let hash = msg_hash(grp.channel_hash, text.as_bytes(), dec.timestamp);
            let already_seen = MSG_SEEN.lock(|cell| {
                let mut ring = cell.borrow_mut();
                if ring.contains(hash) {
                    true
                } else {
                    ring.insert(hash);
                    false
                }
            });
            if already_seen {
                defmt::debug!("GrpTxt: duplicate message suppressed (hash={=u32:#010x})", hash);
                return;
            }

            defmt::info!(
                "MeshCore GrpTxt [{=str} ts={=u32} {=i16}dBm]: {=str}",
                ch.name,
                dec.timestamp,
                rssi,
                text
            );

            let (sender_str, msg_str) = match text.find(": ") {
                Some(i) => (&text[..i], &text[i + 2..]),
                None => ("", text),
            };
            let mut sender: heapless::String<32> = heapless::String::new();
            let _ = sender.push_str(sender_str);
            let mut text_str: heapless::String<128> = heapless::String::new();
            let _ = text_str.push_str(msg_str);
            crate::LAST_LORA_MSG.lock(|cell| {
                *cell.borrow_mut() = Some(crate::LoraMessage {
                    channel: ch.name,
                    sender,
                    text: text_str,
                    timestamp: dec.timestamp,
                    rssi,
                });
            });
            crate::LORA_MSG_SIGNAL.signal(());
        }
        Err(_) => {
            defmt::warn!("GrpTxt: decryption failed on channel {=str}", ch.name);
        }
    }
}

fn log_advert(payload: &[u8], rssi: i16) {
    use meshcore::payload::advert;

    let a = match advert::deserialize(payload) {
        Ok(a) => a,
        Err(_) => {
            defmt::warn!("Advert: failed to parse payload");
            return;
        }
    };

    let sig_ok = meshcore::identity::verify_advert(&a).is_ok();

    if let Some(ref name) = a.name {
        defmt::info!(
            "MeshCore advert [{=i16}dBm] role={=u8} name={=[u8]} sig_ok={=bool}",
            rssi, a.role.to_u8(), &name[..], sig_ok,
        );
    } else {
        defmt::info!(
            "MeshCore advert [{=i16}dBm] role={=u8} key={=[u8]} sig_ok={=bool}",
            rssi, a.role.to_u8(), &a.pub_key[..8], sig_ok,
        );
    }

    // Build name string (used both for contacts and display).
    let mut name_str: heapless::String<32> = heapless::String::new();
    if let Some(ref n) = a.name {
        let _ = name_str.push_str(core::str::from_utf8(n).unwrap_or("?"));
    }

    // Upsert into contacts list so TxtMsg can resolve the sender's name.
    CONTACTS.lock(|cell| cell.borrow_mut().upsert(a.pub_key, name_str.clone()));

    let mut pub_key_hex: heapless::String<16> = heapless::String::new();
    for &b in &a.pub_key[..8] {
        let hi = b >> 4;
        let lo = b & 0xF;
        let _ = pub_key_hex.push(if hi < 10 { (b'0' + hi) as char } else { (b'a' + hi - 10) as char });
        let _ = pub_key_hex.push(if lo < 10 { (b'0' + lo) as char } else { (b'a' + lo - 10) as char });
    }

    crate::LAST_ADVERT.lock(|cell| {
        *cell.borrow_mut() = Some(crate::LastAdvert {
            name: name_str,
            pub_key_hex,
            role: a.role.to_u8(),
            sig_ok,
            rssi,
        });
    });
    crate::ADVERT_SIGNAL.signal(());
}

// ---------------------------------------------------------------------------
// TxtMsg (private message) handler
// ---------------------------------------------------------------------------

fn log_txt_msg(payload: &[u8], rssi: i16, identity: &DeviceIdentity) {
    use meshcore::payload::txt_msg;

    let msg = match txt_msg::deserialize(payload) {
        Ok(m) => m,
        Err(_) => {
            defmt::warn!("TxtMsg: failed to parse payload");
            return;
        }
    };

    // Only process messages addressed to us.
    if msg.dest_pub_key != identity.pub_key {
        defmt::debug!("TxtMsg: not for us, ignoring");
        return;
    }

    // Try to decrypt using each known contact as the potential sender.
    type DecResult = Option<(heapless::String<32>, [u8; meshcore::PUB_KEY_SIZE], meshcore::payload::txt_msg::DecryptedTxtMsg)>;
    let result: DecResult = CONTACTS.lock(|cell| {
        let contacts = cell.borrow();
        for contact in contacts.iter() {
            if txt_msg::verify_mac(&identity.sec_key, &contact.pub_key, &msg).is_ok() {
                if let Ok(dec) = txt_msg::decrypt(&identity.sec_key, &contact.pub_key, &msg) {
                    return Some((contact.name.clone(), contact.pub_key, dec));
                }
            }
        }
        None
    });

    match result {
        None => {
            defmt::warn!("TxtMsg: received but could not decrypt (sender unknown or MAC fail) [{=i16}dBm]", rssi);
        }
        Some((sender_name, sender_pk, dec)) => {
            let text = core::str::from_utf8(&dec.text).unwrap_or("<invalid utf-8>");
            defmt::info!(
                "TxtMsg from {=str} [{=i16}dBm ts={=u32}]: {=str}",
                sender_name.as_str(),
                rssi,
                dec.timestamp,
                text,
            );

            // Fallback name: first 8 bytes of pub_key as hex.
            let display_name = if sender_name.is_empty() {
                let mut hex: heapless::String<32> = heapless::String::new();
                for &b in &sender_pk[..4] {
                    let _ = hex.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('?'));
                    let _ = hex.push(char::from_digit((b & 0xF) as u32, 16).unwrap_or('?'));
                }
                hex
            } else {
                sender_name
            };

            let mut text_str: heapless::String<{ meshcore::payload::txt_msg::MAX_TXT_TEXT_SIZE }> =
                heapless::String::new();
            let _ = text_str.push_str(text);

            crate::LAST_PM.lock(|cell| {
                *cell.borrow_mut() = Some(crate::LastPm {
                    sender_name: display_name,
                    text: text_str,
                    timestamp: dec.timestamp,
                    rssi,
                });
            });
            crate::PM_SIGNAL.signal(());
        }
    }
}

// ---------------------------------------------------------------------------
// Advert transmission
// ---------------------------------------------------------------------------

/// Build and broadcast a signed advert packet for this device.
///
/// `name` is the device name shown to other MeshCore nodes (max 32 bytes).
/// `timestamp` should be a monotonic counter or wall-clock seconds.
pub async fn send_advert(
    lora: &mut SimpleLoRa<'_>,
    identity: &DeviceIdentity,
    name: &[u8],
    timestamp: u32,
) {
    use meshcore::payload::advert::{Advert, DeviceRole, serialize};
    use meshcore::packet::{Message, PayloadType, RouteType};
    use meshcore::{MAX_PAYLOAD_SIZE, MAX_TRANS_UNIT};

    let mut advert = Advert {
        pub_key:   identity.pub_key,
        timestamp,
        signature: [0u8; meshcore::SIGNATURE_SIZE],
        role:      DeviceRole::ChatNode,
        name:      {
            let mut v = heapless::Vec::new();
            let _ = v.extend_from_slice(&name[..name.len().min(32)]);
            if v.is_empty() { None } else { Some(v) }
        },
        position:  None,
        extra1:    None,
        extra2:    None,
    };

    if let Err(e) = meshcore::identity::sign_advert(&identity.sec_key, &mut advert) {
        defmt::warn!("send_advert: signing failed: {:?}", defmt::Debug2Format(&e));
        return;
    }

    let mut payload_buf = [0u8; MAX_PAYLOAD_SIZE];
    let mut payload_len = 0usize;
    if let Err(e) = serialize(&advert, &mut payload_buf, &mut payload_len) {
        defmt::warn!("send_advert: serialize failed: {:?}", defmt::Debug2Format(&e));
        return;
    }

    let mut msg_payload: heapless::Vec<u8, MAX_PAYLOAD_SIZE> = heapless::Vec::new();
    let _ = msg_payload.extend_from_slice(&payload_buf[..payload_len]);

    let msg = Message {
        payload_type:   PayloadType::Advert,
        route:          RouteType::Flood,
        version:        0,
        transport_code: 0,
        path:           heapless::Vec::new(),
        payload:        msg_payload,
    };

    let mut frame = [0u8; MAX_TRANS_UNIT];
    match meshcore::packet::serialize(&msg, &mut frame) {
        Ok(len) => {
            if let Err(e) = lora.send_message(&frame[..len]).await {
                defmt::warn!("send_advert: TX failed: {:?}", e);
            } else {
                defmt::info!("MeshCore advert sent ({=usize}B)", len);
            }
        }
        Err(e) => {
            defmt::warn!("send_advert: packet serialize failed: {:?}", defmt::Debug2Format(&e));
        }
    }
}

// ---------------------------------------------------------------------------
// PM (TxtMsg) transmission
// ---------------------------------------------------------------------------

/// Encrypt and send a private message to `recipient_pk`.
///
/// The recipient must have previously broadcast an advert so their key is
/// known to the mesh.  `text` is plain UTF-8, max [`meshcore::payload::txt_msg::MAX_TXT_TEXT_SIZE`] bytes.
pub async fn send_pm(
    lora: &mut SimpleLoRa<'_>,
    identity: &DeviceIdentity,
    recipient_pk: &[u8; meshcore::PUB_KEY_SIZE],
    text: &[u8],
    timestamp: u32,
) {
    use meshcore::payload::txt_msg;
    use meshcore::packet::{Message, PayloadType, RouteType};
    use meshcore::{MAX_PAYLOAD_SIZE, MAX_TRANS_UNIT};

    let msg = match txt_msg::encrypt(&identity.sec_key, recipient_pk, timestamp, 0, text) {
        Ok(m) => m,
        Err(e) => {
            defmt::warn!("send_pm: encrypt failed: {:?}", defmt::Debug2Format(&e));
            return;
        }
    };

    let mut payload_buf = [0u8; MAX_PAYLOAD_SIZE];
    let mut payload_len = 0usize;
    if let Err(e) = txt_msg::serialize(&msg, &mut payload_buf, &mut payload_len) {
        defmt::warn!("send_pm: serialize failed: {:?}", defmt::Debug2Format(&e));
        return;
    }

    let mut msg_payload: heapless::Vec<u8, MAX_PAYLOAD_SIZE> = heapless::Vec::new();
    let _ = msg_payload.extend_from_slice(&payload_buf[..payload_len]);

    // TxtMsg uses Direct route so the full path to the recipient is embedded.
    // For now we send as Flood — the recipient will filter on dest_pub_key.
    let packet = Message {
        payload_type:   PayloadType::TxtMsg,
        route:          RouteType::Flood,
        version:        0,
        transport_code: 0,
        path:           heapless::Vec::new(),
        payload:        msg_payload,
    };

    let mut frame = [0u8; MAX_TRANS_UNIT];
    match meshcore::packet::serialize(&packet, &mut frame) {
        Ok(len) => {
            if let Err(e) = lora.send_message(&frame[..len]).await {
                defmt::warn!("send_pm: TX failed: {:?}", e);
            } else {
                defmt::info!("PM sent ({=usize}B)", len);
            }
        }
        Err(e) => {
            defmt::warn!("send_pm: packet serialize failed: {:?}", defmt::Debug2Format(&e));
        }
    }
}
