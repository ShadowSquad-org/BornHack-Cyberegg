use super::iso14443::iso14443_3;
use super::iso14443::iso14443_4::{Card, IsoDep};
use defmt::{todo, *};
use embassy_nrf::nfct::NfcT;
use embassy_nrf::nfct::{Config as NfcConfig, NfcId};
use embassy_nrf::peripherals::NFCT;
use embassy_nrf::{Peri, bind_interrupts, nfct};
use {defmt_rtt as _, embassy_nrf as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    NFCT => nfct::InterruptHandler;
});

pub async fn run_nfct(nfct: Peri<'_, NFCT>) {
    dbg!("Setting up...");
    let config = NfcConfig {
        nfcid1: NfcId::DoubleSize([0x04, 0x68, 0x95, 0x71, 0xFA, 0x5C, 0x64]),
        sdd_pat: nfct::SddPat::SDD00100,
        plat_conf: 0b0000,
        protocol: nfct::SelResProtocol::Type4A,
    };

    let mut nfc = NfcT::new(nfct, Irqs, &config);

    let mut buf = [0u8; 256];

    let cc = &[
        0x00, 0x0f, /* CCEN_HI, CCEN_LOW */
        0x20, /* VERSION */
        0x00, 0x7f, /* MLe_HI, MLe_LOW */
        0x00, 0x7f, /* MLc_HI, MLc_LOW */
        /* TLV */
        0x04, 0x06, 0xe1, 0x04, 0x00, 0x7f, 0x00, 0x00,
    ];

    let ndef = &[
        0x00, 0x10, 0xd1, 0x1, 0xc, 0x55, 0x4, 0x65, 0x6d, 0x62, 0x61, 0x73, 0x73, 0x79, 0x2e,
        0x64, 0x65, 0x76,
    ];
    let mut selected: &[u8] = cc;

    loop {
        info!("activating");
        nfc.activate().await;
        info!("activated!");

        let mut nfc = IsoDep::new(iso14443_3::Logger(&mut nfc));

        loop {
            let n = match nfc.receive(&mut buf).await {
                Ok(n) => n,
                Err(e) => {
                    error!("rx error {}", e);
                    break;
                }
            };
            let req = &buf[..n];
            info!("iso-dep rx {:02x}", req);

            let Ok(apdu) = Apdu::parse(req) else {
                error!("apdu parse error");
                break;
            };

            info!("apdu: {:?}", apdu);

            let resp = match (apdu.cla, apdu.ins, apdu.p1, apdu.p2) {
                (0, 0xa4, 4, 0) => {
                    info!("select app");
                    &[0x90, 0x00][..]
                }
                (0, 0xa4, 0, 12) => {
                    info!("select df");
                    match apdu.data {
                        [0xe1, 0x03] => {
                            selected = cc;
                            &[0x90, 0x00][..]
                        }
                        [0xe1, 0x04] => {
                            selected = ndef;
                            &[0x90, 0x00][..]
                        }
                        _ => todo!(), // return NOT FOUND
                    }
                }
                (0, 0xb0, p1, p2) => {
                    info!("read");
                    let offs = u16::from_be_bytes([p1 & 0x7f, p2]) as usize;
                    let len = if apdu.le == 0 {
                        usize::MAX
                    } else {
                        apdu.le as usize
                    };
                    let n = len.min(selected.len() - offs);
                    buf[..n].copy_from_slice(&selected[offs..][..n]);
                    buf[n..][..2].copy_from_slice(&[0x90, 0x00]);
                    &buf[..n + 2]
                }
                _ => {
                    info!("Got unknown command!");
                    &[0xFF, 0xFF]
                }
            };

            info!("iso-dep tx {:02x}", resp);

            match nfc.transmit(resp).await {
                Ok(()) => {}
                Err(e) => {
                    error!("tx error {}", e);
                    break;
                }
            }
        }
    }
}

#[derive(Debug, Clone, defmt::Format)]
struct Apdu<'a> {
    pub cla: u8,
    pub ins: u8,
    pub p1: u8,
    pub p2: u8,
    pub data: &'a [u8],
    pub le: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
struct ApduParseError;

impl<'a> Apdu<'a> {
    pub fn parse(apdu: &'a [u8]) -> Result<Self, ApduParseError> {
        if apdu.len() < 4 {
            return Err(ApduParseError);
        }

        let (data, le) = match apdu.len() - 4 {
            0 => (&[][..], 0),
            1 => (&[][..], apdu[4]),
            n if n == 1 + apdu[4] as usize && apdu[4] != 0 => (&apdu[5..][..apdu[4] as usize], 0),
            n if n == 2 + apdu[4] as usize && apdu[4] != 0 => {
                (&apdu[5..][..apdu[4] as usize], apdu[apdu.len() - 1])
            }
            _ => return Err(ApduParseError),
        };

        Ok(Apdu {
            cla: apdu[0],
            ins: apdu[1],
            p1: apdu[2],
            p2: apdu[3],
            data,
            le: le as _,
        })
    }
}
