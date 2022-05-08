mod dns;
mod ntp;

/// 可利用的UDP放大器
pub trait Amplifier: Send + Sync {
    /// 通过配置文件初始化放大器
    /// 如配置有误，返回相应的错误信息
    fn new(config: serde_json::Value) -> Result<Self, String> where Self: Sized;
    /// 制作一个请求包所需的UDP载荷
    fn make_udp_payload(&self) -> Vec<u8>;
    /// 获取目的IP地址
    fn get_dest_ip(&self) -> String;
    /// 获取目的端口
    fn get_dest_port(&self) -> u16;
}

pub fn get_amplifier(config: serde_json::Value) -> Result<Box<dyn Amplifier>, String> {
    match config["type"].as_str().unwrap() {
        "dns" => Ok(Box::new(dns::DnsAmplifier::new(config)?)),
        "ntp" => Ok(Box::new(ntp::NtpAmplifier::new(config)?)),
        // "quic" => Ok(Box::new(quic::QuicAmplifier::new(config)?)),
        _ => Err(format!("未知的放大器类型: {}", config["type"].as_str().unwrap())),
    }
}