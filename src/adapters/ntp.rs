use std::{net::IpAddr, str::FromStr};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Ntp {
    monlist: bool,
    ip: String,
    port: u16,
}

impl super::AdapterTrait for Ntp {
    fn name(&self) -> &'static str {
        "ntp"
    }

    fn generate_payload(&self) -> Vec<u8> {
        // vec![0u8; 1024]
        // 0x1b + 47 * 0
        if self.monlist {
            let mut payload = vec![0x17, 0x00, 0x03, 0x2a];
            payload.extend(vec![0; 61]);
            payload
        } else {
            let mut payload = vec![0x1b];
            payload.extend(vec![0; 47]);
            payload
        }
    }

    fn dest(&self) -> (IpAddr, u16) {
        (IpAddr::from_str(&self.ip).unwrap(), self.port)
    }
}
