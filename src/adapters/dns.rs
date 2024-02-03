use std::{net::IpAddr, str::FromStr};

use serde::Deserialize;

use super::AdapterTrait;

#[derive(Debug, Deserialize)]
pub struct Query {
    #[serde(rename = "type")]
    type_: String,
    name: String,
}

impl Query {
    /// Only support common types
    fn type_to_id(&self) -> u16 {
        match self.type_.as_str() {
            "A" => 1,
            "NS" => 2,
            "CNAME" => 5,
            "SOA" => 6,
            "PTR" => 12,
            "MX" => 15,
            "TXT" => 16,
            "AAAA" => 28,
            "SRV" => 33,
            "ANY" => 255,
            _ => 0,
        }
    }

    fn generate(&self) -> Vec<u8> {
        let mut result = Vec::new();

        let parts: Vec<&str> = self.name.split('.').collect();
        for part in parts {
            result.push(part.len() as u8);
            result.extend(part.as_bytes());
        }
        result.push(0);

        let id = self.type_to_id();
        result.extend(id.to_be_bytes());

        // class: IN
        result.extend(1u16.to_be_bytes());
        result
    }
}

#[derive(Debug, Deserialize)]
pub struct Dns {
    ip: String,
    port: u16,
    queries: Vec<Query>,
}

impl AdapterTrait for Dns {
    fn name(&self) -> &'static str {
        "dns"
    }

    fn generate_payload(&self) -> Vec<u8> {
        let mut result = Vec::new();

        let transaction_id = 0x1234u16.to_be_bytes();
        result.extend(&transaction_id);
        let flags = 0x0100u16.to_be_bytes();
        result.extend(&flags);

        let query_count = self.queries.len() as u16;
        result.extend(query_count.to_be_bytes());

        result.extend(0u16.to_be_bytes());
        result.extend(0u16.to_be_bytes());
        result.extend(0u16.to_be_bytes());

        for query in &self.queries {
            result.extend(query.generate());
        }

        result
    }

    fn dest(&self) -> (IpAddr, u16) {
        (IpAddr::from_str(&self.ip).unwrap(), self.port)
    }
}
