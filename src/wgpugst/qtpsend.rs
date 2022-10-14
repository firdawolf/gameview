use byte_slice_cast::AsByteSlice;
use qp2p::{Config, Connection, Endpoint};

use bytes::Bytes;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::Duration,
};
use winapi::um::winuser::{
    self, GetSystemMetrics, SetCursorPos, INPUT_MOUSE, SM_CXSCREEN, SM_CYSCREEN,
};
use winit::dpi::PhysicalSize;
use winuser::{INPUT_u, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, MOUSEINPUT};

extern crate gstreamer as gst;
extern crate gstreamer_audio as gst_audio;
extern crate gstreamer_base as gst_base;
extern crate gstreamer_video as gst_video;
use flexbuffers;
use flexbuffers::{BitWidth, Builder, Reader, ReaderError};
use tokio::{self, runtime, sync::broadcast::error::RecvError};

use gst::{
    traits::{ElementExt, GstObjectExt, PadExt},
    Buffer, Element, GstValueExt, Sample,
};

use gst::Structure;
pub async fn send_data<'a>(sample: &'a Sample, sent_stream_input: &'a Connection) {
    let buffer = sample.buffer_owned().expect("cannot get buffer");

    let readmemory = buffer.map_readable().unwrap();
    let framedata = Bytes::copy_from_slice(readmemory.as_byte_slice());
    sent_stream_input
        .send(framedata)
        .await
        .expect("sending data fail");
}

fn press_key_keyboard(keyscan: u16, keyboard_state: u32) {
    let mut input_u: INPUT_u = unsafe { std::mem::zeroed() };
    unsafe {
        *input_u.ki_mut() = KEYBDINPUT {
            wVk: 0,
            dwExtraInfo: 0,
            wScan: keyscan,
            time: 0,
            dwFlags: keyboard_state,
        }
    }
    let mut input = INPUT {
        type_: INPUT_KEYBOARD,
        u: input_u,
    };
    let ipsize = std::mem::size_of::<INPUT>() as i32;
    unsafe {
        SendInput(1, &mut input, ipsize);
    };
}
fn press_key_mouse(flags: u32, positionx: i32, positiony: i32, mouseData: u32) {
    let mut input_u: INPUT_u = unsafe { std::mem::zeroed() };

    match flags {
        winuser::MOUSEEVENTF_MOVE => unsafe {
            SetCursorPos(positionx, positiony);
        },
        winuser::MOUSEEVENTF_WHEEL => unsafe {
            println!("get data");
            *input_u.mi_mut() = MOUSEINPUT {
                dx: positionx,
                dy: positiony,
                dwExtraInfo: 0,
                time: 0,
                dwFlags: flags,
                mouseData: mouseData,
            };
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: input_u,
            };
            let ipsize = std::mem::size_of::<INPUT>() as i32;

            SendInput(1, &mut input, ipsize);
        },
        _ => unsafe {
            *input_u.mi_mut() = MOUSEINPUT {
                dx: positionx,
                dy: positiony,
                dwExtraInfo: 0,
                time: 0,
                dwFlags: flags,
                mouseData: mouseData,
            };
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: input_u,
            };
            let ipsize = std::mem::size_of::<INPUT>() as i32;

            SendInput(1, &mut input, ipsize);
        },
    }
}

fn print_caps(caps: &gst::Caps, prefix: &str) {
    if caps.is_any() {
        println!("{}ANY", prefix);
        return;
    }

    if caps.is_empty() {
        println!("{}EMPTY", prefix);
        return;
    }

    for structure in caps.iter() {
        println!("{}{}", prefix, structure.name());
        for (field, value) in structure.iter() {
            println!(
                "{}  {}:{}",
                prefix,
                field,
                value.serialize().unwrap().as_str()
            );
        }
    }
}

fn print_pad_capabilities(element: &gst::Element, pad_name: &str) {
    let pad = element
        .static_pad(pad_name)
        .expect("Could not retrieve pad");

    println!("Caps for the {} pad:", pad_name);
    let caps = pad.current_caps().unwrap_or_else(|| pad.query_caps(None));
    print_caps(&caps, "      ");
}

#[tokio::main]
pub async fn qtpsend(
    source: Element,
    appsink: gstreamer_app::AppSink,
    pipeline: gst::Pipeline,
    size1: PhysicalSize<u32>,
    framerate: i32,
    pipeline2: gst::Pipeline,
    appsinkaudio: gstreamer_app::AppSink,
    shutdown_rx: Receiver<()>,
) {
    let public_ip = public_ip::addr_v4()
        .await
        .expect("cannot get public ip address");
    let (node, mut incoming_conns, _contact) = Endpoint::new_peer(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 56684)),
        &[],
        Config {
            forward_port: false,

            external_ip: Some(IpAddr::V4(public_ip)),
            external_port: Some(56684 as u16),
            idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
            ..Default::default()
        },
    )
    .await
    .expect("Cannot create endpoint");

    let (_node2, mut incoming_conns2, _contact2) = Endpoint::new_peer(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 56685)),
        &[],
        Config {
            forward_port: false,

            external_ip: Some(IpAddr::V4(public_ip)),
            external_port: Some(56685 as u16),
            idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
            ..Default::default()
        },
    )
    .await
    .expect("Cannot create endpoint");
    println!("create connection 3 ..");
    let (_node3, mut incoming_conns3, _contact3) = Endpoint::new_peer(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 56686)),
        &[],
        Config {
            forward_port: false,

            external_ip: Some(IpAddr::V4(public_ip)),
            external_port: Some(56686 as u16),
            idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
            ..Default::default()
        },
    )
    .await
    .expect("Cannot create endpoint");
    // let (_node4, mut incoming_conns4, _contact4) = Endpoint::new_peer(
    //     SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 56687)),
    //     &[],
    //     Config {
    //         forward_port: false,

    //         external_ip: Some(IpAddr::V4(public_ip)),
    //         external_port: Some(56687 as u16),
    //         idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
    //         ..Default::default()
    //     },
    // )
    // .await
    // .expect("Cannot create endpoint");
    loop {
        println!("Start");
        println!("\n---");
        println!("Listening on: {:?}", node.public_addr());
        println!("---\n");
        println!("Waiting Connection ...");
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        //video connection stream
        let (conn, _incoming_messages) =
            incoming_conns.next().await.expect("cannot get connection");
        //let (mut sendstream, _recvstream) = conn.open_bi().await.expect("cannot open stream");
        //audio connection stream
        let (conn2, _incoming_messages2) =
            incoming_conns2.next().await.expect("cannot get connection");
        //let (mut sendstream2, _recvstream2) = conn2.open_bi().await.expect("cannot open stream");
        //input connection stream
        let (_conn3, mut incoming_messages3) =
            incoming_conns3.next().await.expect("cannot get connection");
        // let (_conn4, mut incoming_messages4) =
        //     incoming_conns4.next().await.expect("cannot get connection");
        // let (_sendstream3, mut recvstream3) = conn3.open_bi().await.expect("cannot open stream");

        //let arcconn = Arc::new(conn);
        println!("Getting to set caps");
        appsink.set_caps(Some(
            &gst::Caps::builder_full()
                .structure(Structure::new(
                    "video/x-h265",
                    &[
                        ("stream-format", &"byte-stream"),
                        ("alignment", &"au"),
                        ("profile", &"main"),
                        ("width", &(size1.width as i32)),   //1476
                        ("height", &(size1.height as i32)), //830
                    ],
                ))
                .build(),
        ));

        appsinkaudio.set_caps(Some(
            &gst::Caps::builder_full()
                .structure(Structure::new(
                    "audio/x-opus",
                    &[
                    //("format", &"S16LE"),
                    //("layout", &"interleaved"),
                    // ("rate", &(16000 as i32)), //1476
                    // ("channels", &(1 as i32)), //830
                    // ("channel-mapping-family", &(0 as i32)),
                    // ("stream-count", &(1 as i32)),
                    // ("coupled-count", &(0 as i32)),
                ],
                ))
                .build(),
        ));
        let connection1arc = Arc::new(conn);
        let connection2arc = Arc::new(conn2);
        let threaded_rt = runtime::Runtime::new().unwrap();
        let threaded_rt_arc = Arc::new(threaded_rt);
        let threaded_rt_clone = Arc::clone(&threaded_rt_arc);
        let threaded_rt_clone2 = Arc::clone(&threaded_rt_arc);
        appsink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                // Add a handler to the "new-sample" signal.
                .new_sample(move |appsink| {
                    match appsink.pull_sample().map_err(|_| gst::FlowError::Eos) {
                        Ok(sample) => {
                            let sent_stream_input = Arc::clone(&connection1arc);

                            // thread::spawn(move || {
                            //     pollster::block_on(async {
                            //         send_data(&sample, &sent_stream_input).await;
                            //     });
                            // });
                            threaded_rt_clone
                                .spawn(async move { send_data(&sample, &sent_stream_input).await });
                        }
                        Err(err) => println!("sample cannot be input {}", err),
                    };

                    // Pull the sample in question out of the appsink's buffer.

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
        appsinkaudio.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                // Add a handler to the "new-sample" signal.
                .new_sample(move |appsinkaudio2| {
                    match appsinkaudio2.pull_sample().map_err(|_| gst::FlowError::Eos) {
                        Ok(sample) => {
                            let sent_stream_input = Arc::clone(&connection2arc);

                            threaded_rt_clone2
                                .spawn(async move { send_data(&sample, &sent_stream_input).await });
                        }
                        Err(err) => println!("sample cannot be input {}", err),
                    };

                    // Pull the sample in question out of the appsink's buffer.

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        print_pad_capabilities(&source, "src");
        tokio::spawn(async move {
            loop {
                let bytes1 = incoming_messages3.next().await;
                match bytes1 {
                    Ok(bytes) => {
                        if bytes != None {
                            let data = bytes.expect("cannot get byte");
                            let root = Reader::get_root(data.as_byte_slice()).unwrap();
                            let read_input = root.as_map();
                            let mouse_state = read_input.idx("mouse_state").as_u32();
                            let mouse_data = read_input.idx("mouse_data").as_u32();
                            let position_x = read_input.idx("position_x").as_i32();
                            let position_y = read_input.idx("position_y").as_i32();
                            let keyscan = read_input.idx("keyscan").as_u16();
                            let keyboard_state = read_input.idx("keyboard_state").as_u32();
                            // println!(
                            //     "Mouse state :{} x :{} y :{}",
                            //     mouse_state, position_x, position_y
                            // );
                            if position_x != 0 && position_y != 0 {
                                press_key_mouse(mouse_state, position_x, position_y, mouse_data);
                            } else if keyscan != 0 && keyboard_state != 0 {
                                press_key_keyboard(keyscan, keyboard_state)
                            }
                        }
                    }
                    Err(e) => {
                        sender.send("stop").await.unwrap();
                        break;
                    }
                }
            }
        });

        pipeline
            .set_state(gst::State::Playing)
            .expect("cannot set ready");
        pipeline2
            .set_state(gst::State::Playing)
            .expect("cannot set ready");
        println!("In Running state ...");
        //let bus = pipeline2.bus().unwrap();

        loop {
            if receiver.recv().await.unwrap() == "stop" {
                break;
            } else {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
    //node.close();
}
