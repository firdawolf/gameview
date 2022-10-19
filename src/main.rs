use clap::Parser;

mod wgpugst;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Argscustom {
    /// Sent or Receive
    #[arg(short, long)]
    pub sent_or_receive: String,
    /// (SENT) the monitor index
    #[arg(short, long, default_value_t = 0)]
    pub monitor: i32,
    /// (SENT) show cursor or not
    #[arg(short, long, default_value_t = false)]
    pub show_cursor: bool,
    /// (SENT) Encode with bframes
    #[arg(short, long, default_value_t = 0)]
    pub bframes: u32,
    /// (SENT) Encode with bframes
    #[arg(short, long, default_value_t = 5000)]
    pub bitrate: u32,
    /// (SENT) Higher number =Better quality, Lower number =Faster speed Value from : 100 - 10
    #[arg(short, long, default_value_t = 50)]
    pub quality_vs_speed: u32,
    /// (SENT) Mode : cbr for constant bitrate or vbr for variable bitrate
    #[arg(short, long, default_value_t = String::from("cbr"))]
    pub rc_mode: String,
    /// (SENT) true for low-latency with trade off quality
    #[arg(short, long, default_value_t = false)]
    pub low_latency: bool,
    /// (SENT) Downscale percent for use with fsr 0 - 40
    #[arg(short, long, default_value_t = 0)]
    pub downscale: i32,
    /// (RECEIVE) IP to connect to
    #[arg(short, long, default_value_t = String::from("127.0.0.1:56687"))]
    pub ip_connect: String,
}

fn main() {
    //service_dispatcher::start("wgpugst", ffi_service_main).unwrap();
    let args = Argscustom::parse();

    if args.sent_or_receive == "sent".to_string() {
        wgpugst::sent(args);
    } else if args.sent_or_receive == "receive".to_string() {
        wgpugst::receive(args);
    } else {
        println!("choose argument of (sent) or (receive)")
    }
}
