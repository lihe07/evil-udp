use std::collections::HashMap;
use std::str::FromStr;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::Value;
use super::Amplifier;

#[derive(Debug, Deserialize)]
struct Query {
    #[serde(rename = "name")]
    query_name: String,
    #[serde(rename = "type")]
    query_type: String,
}

lazy_static! {
    static ref QUERY_TYPE: HashMap<String, u16> = {
        let mut m = HashMap::new();
        // DNS Query Types (https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml#dns-parameters-4)
        m.insert("SIGZERO".to_string(), 0);
        m.insert("A".to_string(), 1);
        m.insert("NS".to_string(), 2);
        m.insert("MD".to_string(), 3);
        m.insert("MF".to_string(), 4);
        m.insert("CNAME".to_string(), 5);
        m.insert("SOA".to_string(), 6);
        m.insert("MB".to_string(), 7);
        m.insert("MG".to_string(), 8);
        m.insert("MR".to_string(), 9);
        m.insert("NULL".to_string(), 10);
        m.insert("WKS".to_string(), 11);
        m.insert("PTR".to_string(), 12);
        m.insert("HINFO".to_string(), 13);
        m.insert("MINFO".to_string(), 14);
        m.insert("MX".to_string(), 15);
        m.insert("TXT".to_string(), 16);
        m.insert("RP".to_string(), 17);
        m.insert("AFSDB".to_string(), 18);
        m.insert("X25".to_string(), 19);
        m.insert("ISDN".to_string(), 20);
        m.insert("RT".to_string(), 21);
        m.insert("NSAP".to_string(), 22);
        m.insert("NSAPPTR".to_string(), 23);
        m.insert("SIG".to_string(), 24);
        m.insert("KEY".to_string(), 25);
        m.insert("PX".to_string(), 26);
        m.insert("GPOS".to_string(), 27);
        m.insert("AAAA".to_string(), 28);
        m.insert("LOC".to_string(), 29);
        m.insert("NXT".to_string(), 30);
        m.insert("EID".to_string(), 31);
        m.insert("NIMLOC".to_string(), 32);
        m.insert("SRV".to_string(), 33);
        m.insert("ATMA".to_string(), 34);
        m.insert("NAPTR".to_string(), 35);
        m.insert("KX".to_string(), 36);
        m.insert("CERT".to_string(), 37);
        m.insert("A6".to_string(), 38);
        m.insert("DNAME".to_string(), 39);
        m.insert("SINK".to_string(), 40);
        m.insert("OPT".to_string(), 41);
        m.insert("APL".to_string(), 42);
        m.insert("DS".to_string(), 43);
        m.insert("SSHFP".to_string(), 44);
        m.insert("IPSECKEY".to_string(), 45);
        m.insert("RRSIG".to_string(), 46);
        m.insert("NSEC".to_string(), 47);
        m.insert("DNSKEY".to_string(), 48);
        m.insert("DHCID".to_string(), 49);
        m.insert("NSEC3".to_string(), 50);
        m.insert("NSEC3PARAM".to_string(), 51);
        m.insert("TLSA".to_string(), 52);
        m.insert("SMIMEA".to_string(), 53);
        m.insert("HIP".to_string(), 55);
        m.insert("NINFO".to_string(), 56);
        m.insert("RKEY".to_string(), 57);
        m.insert("TALINK".to_string(), 58);
        m.insert("CDS".to_string(), 59);
        m.insert("CDNSKEY".to_string(), 60);
        m.insert("OPENPGPKEY".to_string(), 61);
        m.insert("CSYNC".to_string(), 62);
        m.insert("SPF".to_string(), 99);
        m.insert("UINFO".to_string(), 100);
        m.insert("UID".to_string(), 101);
        m.insert("GID".to_string(), 102);
        m.insert("UNSPEC".to_string(), 103);
        m.insert("NID".to_string(), 104);
        m.insert("L32".to_string(), 105);
        m.insert("L64".to_string(), 106);
        m.insert("LP".to_string(), 107);
        m.insert("EUI48".to_string(), 108);
        m.insert("EUI64".to_string(), 109);
        m.insert("TKEY".to_string(), 249);
        m.insert("TSIG".to_string(), 250);
        m.insert("IXFR".to_string(), 251);
        m.insert("AXFR".to_string(), 252);
        m.insert("MAILB".to_string(), 253);
        m.insert("MAILA".to_string(), 254);
        m.insert("ANY".to_string(), 255);
        m
    };
}

impl Query {
    /// 检查这个查询是否是一个有效的查询
    /// 如果有效，则返回None
    /// 如果无效，则返回错误信息
    fn check(&self) -> Option<String> {
        if !QUERY_TYPE.contains_key(&self.query_type) {
            return Some("无效的查询类型: ".to_string() + &self.query_type);
        }
        for part in self.query_name.split(".") {
            if part.len() > 255 {
                return Some("域名中每一个部分的长度不能超过255".to_string());
            }
            if !part.is_ascii() {
                return Some("域名中的每一个部分都必须是ASCII字符".to_string());
            }
        }
        None
    }
    fn bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // 查询名称
        let parts = self.query_name.split('.');
        for part in parts {
            buf.push(part.len() as u8);
            buf.extend_from_slice(part.as_bytes());
        }
        // 以0结尾
        buf.push(0);
        // 查询类型
        let query_type = QUERY_TYPE.get(self.query_type.as_str()).unwrap();
        buf.append(&mut query_type.to_be_bytes().to_vec());
        // 查询类 通常为1 表示IN
        buf.append(&mut 1u16.to_be_bytes().to_vec());
        buf
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    ip: String,
    port: u16,
    queries: Vec<Query>,
}

impl Config {
    /// 检验配置文件的有效性
    /// 如果有效，则返回None
    /// 如果无效，则返回错误信息
    fn check(&self) -> Option<String> {
        if self.ip.len() > 255 {
            return Some("IP地址的长度不能超过255".to_string());
        }
        if !self.ip.is_ascii() {
            return Some("IP地址必须是ASCII字符".to_string());
        }
        // 由于目前只支持IPv4，所以检查是否是IPv4
        if let Err(_) = std::net::Ipv4Addr::from_str(&self.ip) {
            return Some("IP地址必须是IPv4".to_string());
        }
        if self.queries.len() == 0 {
            return Some("必须至少配置一个查询".to_string());
        }
        if self.queries.len() > 65535 {
            return Some("查询的数量不能超过65535个".to_string());
        }
        for query in &self.queries {
            if let Some(err) = query.check() {
                return Some(err);
            }
        }
        None
    }
}

pub struct DnsAmplifier {
    config: Config,
}

impl Amplifier for DnsAmplifier {
    fn new(config: Value) -> Result<Self, String> {
        if let Ok(config) = serde_json::from_value::<Config>(config) {
            if let Some(err) = config.check() {
                Err(err)
            } else {
                Ok(DnsAmplifier {
                    config
                })
            }
        } else {
            Err("配置文件不是有效的JSON".to_string())
        }
    }

    fn make_udp_payload(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // 随机生成一个ID
        // 事务ID(Transcation ID) 固定为0
        buf.append(&mut rand::random::<u16>().to_be_bytes().to_vec());
        // 查询标志(Flag) 0x0100 标准递归查询
        buf.append(&mut 0x0100u16.to_be_bytes().to_vec());
        // 问题数量(Question Count)
        buf.append(&mut (self.config.queries.len() as u16).to_be_bytes().to_vec());
        // 回答数量(Answer Count)
        buf.append(&mut 0u16.to_be_bytes().to_vec());
        // 授权数量(Authority Count)
        buf.append(&mut 0u16.to_be_bytes().to_vec());
        // 附加数量(Additional Count)
        buf.append(&mut 0u16.to_be_bytes().to_vec());
        // 问题
        for query in &self.config.queries {
            buf.append(&mut query.bytes());
        }
        buf
    }

    fn get_dest_ip(&self) -> String {
        self.config.ip.to_owned()
    }

    fn get_dest_port(&self) -> u16 {
        self.config.port
    }
}