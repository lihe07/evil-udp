mod adapters;
mod test;
mod udp;

use std::path::PathBuf;

use clap::Parser;
use log::error;

#[derive(Debug, clap::Parser)]
enum App {
    Test(Test),
}

#[derive(Debug, clap::Args)]
struct Test {
    amplifiers: PathBuf,
    #[clap(short, long, help = "Device to use, otherwise default is used.")]
    device: Option<String>,
    #[clap(
        short,
        long,
        help = "Number of packets sent to each adapter.",
        default_value = "10"
    )]
    num_packets: usize,
    #[clap(short, long, help = "IP to bind to.")]
    ip: Option<String>,
}

fn main() {
    pretty_env_logger::init();
    let app = App::parse();
    println!("{:?}", app);

    let res = match app {
        App::Test(args) => smol::block_on(test::test(args)),
    };
    if let Err(e) = res {
        error!("Error: {}", e);
    }
}
