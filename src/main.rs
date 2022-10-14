use std::sync::mpsc;
use std::{env, net::SocketAddr, thread, time::Duration};
#[macro_use]
extern crate windows_service;

use std::ffi::OsString;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::ServiceControlHandlerResult;
use windows_service::{service_control_handler, service_dispatcher};
define_windows_service!(ffi_service_main, my_service_main);
mod wgpugst;

fn my_service_main(arguments: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler
    let status_handle = service_control_handler::register("wgpugst", event_handler);

    let next_status = ServiceStatus {
        // Should match the one from system service registry
        service_type: ServiceType::OWN_PROCESS,
        // The new state
        current_state: ServiceState::Running,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint: 0,
        // Only used for pending states, otherwise must be zero
        wait_hint: Duration::default(),
        process_id: None,
    };
    let statushand = status_handle.unwrap();
    // Tell the system that the service is running now
    statushand.set_service_status(next_status).unwrap();

    //do work
    wgpugst::sent(3500, shutdown_rx);

    //stop service
    statushand
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .unwrap();
}

fn main() {
    //service_dispatcher::start("wgpugst", ffi_service_main).unwrap();
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let args: Vec<String> = env::args().collect();
    let sentorreceive = args[1].parse::<String>().expect("cannot convert to no");
    // println!("Starting Operation in 6 seconds");
    // thread::sleep(Duration::from_secs(6));
    if sentorreceive == "sent".to_string() {
        // let height = args[2].parse::<i32>().expect("cannot convert variable height to integer");
        // let width = args[3].parse::<i32>().expect("cannot convert variable width to integer");
        // let monitor = args[4].parse::<i32>().expect("cannot convert variable monitor Index to integer");
        // let showcursor = args[5].parse::<bool>().expect("cannot convert variable show-cursor with accept true or false");
        let bitrate = args[2]
            .parse::<u32>()
            .expect("cannot convert variable bitrate to u32");
        // let qualityspeed = args[7].parse::<u32>().expect("cannot convert variable quality-vs-speed to integer");
        // let lowlatency = args[8].parse::<bool>().expect("cannot convert variable low-latency with accept true or false");

        wgpugst::sent(bitrate, shutdown_rx);
    } else if sentorreceive == "receive".to_string() {
        let connect_to = args[2]
            .parse::<SocketAddr>()
            .expect("Invalid SocketAddr.  Use the form 127.0.0.1:1234");
        wgpugst::receive(connect_to);
    } else {
        println!("choose argument of (sent) or (receive)")
    }
}
