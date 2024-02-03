mod dns;
mod ntp;
use std::{net::IpAddr, path::Path};

use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait AdapterTrait {
    fn name(&self) -> &'static str;
    fn generate_payload(&self) -> Vec<u8>;
    fn dest(&self) -> (IpAddr, u16);
}

#[enum_dispatch(AdapterTrait)]
pub enum Adapter {
    Ntp(ntp::Ntp),
    Dns(dns::Dns),
}

pub fn read_adapters<P: AsRef<Path>>(path: P) -> Result<Vec<Adapter>, Box<dyn std::error::Error>> {
    let adapters = std::fs::read_to_string(path)?;
    let adapters: Vec<serde_json::Value> = serde_json::from_str(&adapters)?;
    let mut result = Vec::new();
    for adapter in adapters {
        let t = adapter["type"].as_str().ok_or("type not found")?;
        let adapter = match t {
            "ntp" => Adapter::Ntp(serde_json::from_value(adapter)?),
            "dns" => Adapter::Dns(serde_json::from_value(adapter)?),
            // _ => return Err(format!("unknown adapter type: {t}").into()),
            _ => continue,
        };
        result.push(adapter);
    }
    Ok(result)
}
