use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::mpsc::Sender;

use log::{debug, error, info, warn};
use smol::net::UdpSocket;
use smol::Executor;

use crate::adapters::AdapterTrait;
use crate::adapters::{self, Adapter};

/// Find an available port to listen on. (UDP)
async fn listen_arbitrary() -> Result<(u16, UdpSocket), Box<dyn std::error::Error>> {
    for port in 1024..65535 {
        match UdpSocket::bind(format!("[::]:{}", port)).await {
            Ok(socket) => {
                debug!("Listening on port {}", port);
                return Ok((port, socket));
            }
            Err(_) => continue,
        }
    }
    Err("No available ports found".into())
}

async fn listener_thread(name: &'static str, sock: UdpSocket, tx: Sender<(&'static str, usize)>) {
    let mut buf = [0; 1024];
    loop {
        match sock.recv_from(&mut buf).await {
            Ok((size, _)) => {
                debug!("Received {} bytes on {}", size, name);
                tx.send((name, size)).unwrap();
            }
            Err(e) => {
                info!("Error receiving on {}: {}", name, e);
                break;
            }
        }
    }
}

async fn sender_thread(
    adapter: Adapter,
    listener_ip: std::net::IpAddr,
    listener_port: u16,
    tx: Sender<(&'static str, usize)>,
    num: usize,
) {
    debug!("Starting sender thread for {}", adapter.name());

    let mut sender = crate::udp::prepare_sender();
    let name = adapter.name();
    let (ip, port) = adapter.dest();

    for _ in 0..num {
        let payload = adapter.generate_payload();
        let size =
            crate::udp::send_raw_pack(&mut sender, &payload, listener_ip, listener_port, ip, port);
        tx.send((name, size)).unwrap();
    }

    debug!("Sender thread done");
}

pub async fn test(args: crate::Test) -> Result<(), Box<dyn std::error::Error>> {
    let adapters = adapters::read_adapters(args.amplifiers)?;
    let mut adapter_types = HashSet::new();
    let mut stats = HashMap::new();

    let host_ip = if let Some(ip) = args.ip {
        IpAddr::from_str(&ip)?
    } else if let Some(ip) = crate::udp::get_ip() {
        warn!("No host IP specified. Using {}", ip);
        ip
    } else {
        return Err("No host IP specified and unable to determine local IP".into());
    };

    for adapter in &adapters {
        adapter_types.insert(adapter.name());
        stats.insert(adapter.name(), 0);
    }

    // Starting listener threads
    let (tx, rx) = std::sync::mpsc::channel();
    let ex = Executor::new();

    let mut type_to_port = HashMap::new();
    let mut listener_tasks = Vec::new();
    for name in &adapter_types {
        let (port, sock) = listen_arbitrary().await?;
        let tx = tx.clone();
        let name = *name;
        type_to_port.insert(name, port);
        listener_tasks.push(ex.spawn(listener_thread(name, sock, tx)));
    }
    drop(tx);

    let (sender_tx, sender_rx) = std::sync::mpsc::channel();
    let mut sender_tasks = Vec::new();
    for adapter in adapters {
        let tx = sender_tx.clone();
        let listener_ip = host_ip;
        let port = type_to_port[adapter.name()];
        sender_tasks.push(ex.spawn(sender_thread(
            adapter,
            listener_ip,
            port,
            tx,
            args.num_packets,
        )));
    }
    drop(sender_tx);

    info!("Begin executor");
    std::thread::spawn(move || smol::future::block_on(ex.run(smol::future::pending::<()>())));

    info!("Waiting for sender threads to finish");

    let mut total_sent = 0;

    for p in sender_rx {
        debug!("Sent {} bytes", p.1);
        total_sent += p.1;
    }
    info!("Done. Total sent: {} bytes.", total_sent);

    // wait for 3 seconds
    for i in 0..3 {
        info!("Stop in {}...", 3 - i);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    for task in listener_tasks {
        task.cancel().await;
    }
    // Collect stats
    for (name, size) in rx {
        let entry = stats.entry(name).or_insert(0);
        *entry += size;
    }

    for (name, size) in &stats {
        info!("Received {} bytes via {}", size, name);
    }

    Ok(())
}
