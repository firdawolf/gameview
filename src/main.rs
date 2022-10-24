mod wgpugst;
use clap::Parser;
fn main() {
    //service_dispatcher::start("wgpugst", ffi_service_main).unwrap();
    let args = wgpugst::Argscustom::parse();

    if args.sent_or_receive == "sent".to_string() {
        wgpugst::sent(args);
    } else if args.sent_or_receive == "receive".to_string() {
        wgpugst::receive(args);

        // wgpugst::device()
    } else {
        println!("choose argument of (sent) or (receive)")
    }
}
