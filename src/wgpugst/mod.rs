use byte_slice_cast::AsByteSlice;
use bytes::Bytes;
use clap::Arg;
use haphazard::{AtomicPtr, HazardPointer};
use parking_lot::Mutex;
use qp2p::{Config, Connection, ConnectionIncoming, Endpoint, SendStream};
use tokio::{task, time::Instant};
use wgpu::{Texture, TextureView};
use winapi::um::winuser::{
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_MOVE_NOCOALESCE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL,
};
extern crate pollster;

use core::time;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{mpsc::Receiver, Arc},
    thread,
    time::Duration,
};
mod qtpreceive;
mod qtpsend;
use qtpreceive::qtpreceive;
use qtpsend::qtpsend;
extern crate gstreamer as gst;
extern crate gstreamer_audio as gst_audio;
extern crate gstreamer_base as gst_base;
extern crate gstreamer_video as gst_video;

use gst::{glib::Type, prelude::*, CapsFeatures};

use gst::Element;
use iced_native::{
    command::Command,
    futures,
    program::Program,
    renderer,
    widget::{button, column::Column, row::Row, slider, text::Text},
    Alignment, Color, Debug, Length, Size,
};
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};

use iced_winit::{conversion, winit, Clipboard, Error};
use winit::dpi::PhysicalPosition;
use winit::event::ModifiersState;

use flexbuffers;
use flexbuffers::{BitWidth, Builder, Reader, ReaderError};
use winit::monitor::{MonitorHandle, VideoMode};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorIcon, Fullscreen, Window, WindowAttributes},
};

// mod menu;
// use menu::menu_ui::{Gameview, Message};
extern crate gstreamer_app;
use clap::Parser;
use std::i32;

use gst::Structure;

//all public function
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
    /// (SENT) (RECEIVE) The key to use during receive
    #[arg(short, long)]
    pub key: String,
    /// (SENT) (RECEIVE) The password to access during the receive
    #[arg(short, long)]
    pub password: String,
    /// (SENT) true for low-latency with trade off quality
    #[arg(short, long, default_value_t = false)]
    pub low_latency: bool,
    /// (SENT) Downscale percent for use with fsr 0 - 40
    #[arg(short, long, default_value_t = 0)]
    pub downscale: i32,
    /// (RECEIVE) IP to connect to
    #[arg(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 56687))]
    pub ip_connect: SocketAddr,
}

pub fn device() -> String {
    // gst::init().unwrap();
    let monitor = gst::DeviceMonitor::new();
    let bus = monitor.bus();
    let caps = gst::Caps::builder("audio/x-raw").build();
    let mut id = String::from("");
    monitor.add_filter(Some("Audio/Sink"), Some(&caps));
    monitor.start().unwrap();
    monitor.devices().for_each(|device| {
        let spec = device.properties();

        // if spec {
        //     println!("this one is wasapi2");
        // }
        // spec.iter().for_each(|spec| {
        //     println!("Found {}", spec.name());
        // });
        if spec != None {
            let name = spec.unwrap();
            match name.value("device.api") {
                Ok(value) => {
                    if value.serialize().unwrap().as_str() == "wasapi2"
                        && name
                            .value("device.default")
                            .unwrap()
                            .serialize()
                            .unwrap()
                            .as_str()
                            == "true"
                    {
                        id = name
                            .value("device.id")
                            .unwrap()
                            .serialize()
                            .unwrap()
                            .as_str()
                            .to_owned();
                    }
                }
                Err(_) => {}
            }
            // name.fields().for_each(|info| {
            //     println!("Found {}", info);
            // });
        }
    });
    id
}

pub fn sent(args: Argscustom) {
    // Initialize GStreamer
    gst::init().unwrap();

    // gst-inspect-1.0.exe mediafoundation
    // Create the elements
    let event_loop = EventLoop::new();
    // let window = winit::window::Window::new(&event_loop).unwrap();
    let mut monitors = vec![];
    event_loop.available_monitors().for_each(|handle| {
        monitors.push(handle);
    });
    let monitorinfo = &monitors[args.monitor as usize];
    let mut framerate: i32 = 60;
    monitorinfo.video_modes().for_each(|mode| {
        println!("Refresh rate :{}", mode.refresh_rate());
        framerate = mode.refresh_rate() as i32
    });
    let mut size1 = PhysicalSize {
        height: monitorinfo.size().height, // 864
        width: monitorinfo.size().width,   // 1536
    };
    if args.downscale > 0 {
        size1.width = ((size1.width as i32 * (100 - args.downscale)) / 100) as u32;
        size1.height = ((size1.height as i32 * (100 - args.downscale)) / 100) as u32;
    }

    // let audiosource = gst::ElementFactory::make("wasapi2src", Some("audiosource"))
    //     .expect("Could not create audiosource element.");
    // let audioconvert = gst::ElementFactory::make("audioconvert", Some("audioconvert"))
    //     .expect("Could not create audiosource element.");
    // let audioresample = gst::ElementFactory::make("audioresample", Some("audioresample"))
    //     .expect("Could not create audiofilter element");

    // let audiofilter = gst::ElementFactory::make("capsfilter", Some("audiofilter"))
    //     .expect("Could not create audiofilter element");
    // let opusenc = gst::ElementFactory::make("opusenc", Some("opusenc"))
    //     .expect("Could not create opusenc element.");
    // let qtpaudiosink = gst::ElementFactory::make("appsink", Some("qtpaudiosink"))
    //     .expect("Could not create sink element");

    let source = gst::ElementFactory::make("d3d11screencapturesrc", Some("source"))
        .expect("Could not create source element.");
    // let filter = gst::ElementFactory::make("capsfilter", Some("filter"))
    //     .expect("Could not create sink element");
    let convert = gst::ElementFactory::make("d3d11convert", Some("convert"))
        .expect("Could not create sink element");
    // let queue =
    //     gst::ElementFactory::make("queue", Some("queue")).expect("Could not create sink element");
    // let queue2 =
    //     gst::ElementFactory::make("queue", Some("queue2")).expect("Could not create sink element");
    let filter2 = gst::ElementFactory::make("capsfilter", Some("filter2"))
        .expect("Could not create sink element");
    let encode = gst::ElementFactory::make("mfh265enc", Some("encode"))
        .expect("Could not create sink element");
    let filter3 = gst::ElementFactory::make("capsfilter", Some("filter3"))
        .expect("Could not create sink element");
    // let download = gst::ElementFactory::make("d3d11download", Some("decode"))
    //     .expect("Could not create sink element");
    let qtpsink = gst::ElementFactory::make("appsink", Some("qtpsink"))
        .expect("Could not create sink element");

    // let size2 = PhysicalSize {
    //     height: 1080,
    //     width: 1920,
    // };

    // Create the empty pipeline
    let pipeline = gst::Pipeline::new(Some("video-pipeline"));
    // let pipeline2 = gst::Pipeline::new(Some("audio-pipeline"));
    let mut firststring = String::from(r"wasapi2src device=");
    let idstring = &device();
    let secondstring = r" loopback=true ! audioresample ! audioconvert ! queue ! audio/x-raw,rate=24000 ! opusenc ! appsink";
    firststring.push_str(idstring);
    firststring.push_str(secondstring);
    let pipeline_audio_enc = gst::parse_launch(&firststring).unwrap();

    let pipeline_audio = pipeline_audio_enc.dynamic_cast::<gst::Pipeline>().unwrap();
    // pipeline
    //     .add_many(&[&source, &convert, &filter2, &encode, &decode, &sink])
    //     .unwrap();

    pipeline
        .add_many(&[&source, &convert, &filter2, &encode, &qtpsink])
        .unwrap();
    gst::Element::link_many(&[&convert, &filter2, &encode, &qtpsink])
        .expect("Elements could not be linked.");
    let opusenc = pipeline_audio
        .by_name("opusenc0")
        .expect("cannot set opusenc");
    let qtpaudiosink = pipeline_audio
        .by_name("appsink0")
        .expect("cannot set qtpaudiosink");
    // let qtpaudiosrc = pipeline_audio
    // .by_name("appsrc0")
    // .expect("cannot set qtpaudiosink");
    // pipeline2
    //     .add_many(&[
    //         &audiosource,
    //         &audioresample,
    //         &audioconvert,
    //         //&audiofilter,
    //         &opusenc,
    //         &qtpaudiosink,
    //     ])
    //     .unwrap();
    // gst::Element::link_many(&[
    //     &audioresample,
    //     &audioconvert,
    //     //&audiofilter,
    //     &opusenc,
    //     &qtpaudiosink,
    // ])
    // .expect("Elements could not be linked.");

    let filter_caps2 =
        gst::Caps::builder_full_with_features(CapsFeatures::new(&["memory:D3D11Memory"]))
            .structure_with_features(
                Structure::new(
                    "video/x-raw",
                    &[
                        ("format", &"NV12"),
                        ("width", &(size1.width as i32)),   //1476
                        ("height", &(size1.height as i32)), //830
                        ("framerate", &gst::Fraction::new(framerate, 1)),
                    ],
                ),
                CapsFeatures::new(&["memory:D3D11Memory"]),
            )
            .build();
    // let audio_filter = &gst::Caps::builder_full()
    //     .structure(Structure::new(
    //         "audio/x-raw",
    //         &[
    //             ("format", &"S16LE"),
    //             ("layout", &"interleaved"),
    //             ("rate", &(48000 as i32)), //1476
    //             ("channels", &(1 as i32)), //830
    //         ],
    //     ))
    //     .build();
    let filter_caps3 =
        gst::Caps::builder_full_with_features(CapsFeatures::new(&["memory:SystemMemory"]))
            .structure_with_features(
                Structure::new(
                    "video/x-raw",
                    &[
                        ("format", &"NV12"),
                        ("width", &(size1.width as i32)),   //1476
                        ("height", &(size1.height as i32)), //830
                        ("framerate", &gst::Fraction::new(framerate, 1)),
                    ],
                ),
                CapsFeatures::new(&["memory:SystemMemory"]),
            )
            .build();
    // source.set_property("monitor-index", 0 - 1 as u8);
    source.set_property("show-cursor", args.show_cursor as bool);
    encode.set_property("bframes", args.bframes as u32);
    encode.set_property("max-bitrate", args.bitrate as u32);
    encode.set_property("bitrate", args.bitrate as u32);
    encode.set_property("quality-vs-speed", args.quality_vs_speed as u32);
    encode.set_property_from_str("rc-mode", args.rc_mode.as_str());
    // encode.set_property("min-qp", 10 as u32);
    // encode.set_property("max-qp", 15 as u32);
    //encode.set_property("qp", 15 as u32);

    encode.set_property("low-latency", args.low_latency as bool);
    filter2.set_property("caps", filter_caps2);
    filter3.set_property("caps", filter_caps3);

    // audiosource
    //     .try_set_properties(&[
    //         //("device", &r"\\\\\?\\SWD\#MMDEVAPI\#\{0.0.0.00000000\}.\{2b5bf277-dad7-4141-9849-5567e5843362\}\#\{e6327cad-dcec-4949-ae8a-991e976a79d2\}"),
    //         ("loopback", &(true as bool)), //1476
    //         ("low-latency", &(true as bool)),
    //     ])
    //     .expect("cannot set values for audio source properties");

    // audiofilter.set_property("caps", audio_filter);
    opusenc.set_property("bitrate", 64000 as i32);
    opusenc.set_property_from_str("frame-size", "10");
    opusenc.set_property_from_str("bitrate-type", "cbr");
    // opusenc.set_property_from_str("audio-type", "restricted-lowdelay");
    // opusenc.set_property_from_str("bandwidth", "mediumband");
    // opusenc.set_property("inband-fec", true as bool);
    //opusenc.set_property("dtx", true as bool);

    // println!("In NULL state:");

    match source.link(&convert) {
        Ok(_v) => println!("link pad success"),
        Err(_e) => println!("link is not success"),
    }
    // match audiosource.link(&audioresample) {
    //     Ok(_v) => println!("link audio pad success"),
    //     Err(_e) => println!("link is not success"),
    // }
    let qtp2sink = qtpsink
        .dynamic_cast::<gstreamer_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");
    let qtp2audiosink = qtpaudiosink
        .dynamic_cast::<gstreamer_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");
    qtpsend::qtpsend(
        source,
        qtp2sink,
        pipeline,
        size1,
        framerate,
        pipeline_audio,
        qtp2audiosink,
        args.downscale as u32,
        args.key,
        args.password,
    );

    // tokio::runtime::Builder::new_current_thread()
    //     .enable_all()
    //     .build()
    //     .unwrap()
    //     .block_on(async {
    //         qtpsend(source, qtp2sink, pipeline, size1).await;
    //     });
}

#[tokio::main]
pub async fn receive(args: Argscustom) {
    // Initialize GStreamer
    gst::init().unwrap();

    let qp2psrc = gst::ElementFactory::make("appsrc", Some("source"))
        .expect("Could not create source element.");
    let filter2 = gst::ElementFactory::make("capsfilter", Some("filter2"))
        .expect("Could not create sink element");
    // let queue =
    //     gst::ElementFactory::make("queue", Some("queue")).expect("Could not create sink element");
    let decode = gst::ElementFactory::make("d3d11h265dec", Some("decode"))
        .expect("Could not create sink element");
    // let upload = gst::ElementFactory::make("d3d11upload", Some("upload"))
    //     .expect("Could not create sink element");
    let filter3 = gst::ElementFactory::make("capsfilter", Some("filter3"))
        .expect("Could not create sink element");
    let sink =
        gst::ElementFactory::make("appsink", Some("sink")).expect("Could not create sink element");
    let queue =
        gst::ElementFactory::make("queue", Some("queue")).expect("Could not create queue element");
    let download = gst::ElementFactory::make("d3d11download", Some("d3d11download"))
        .expect("Could not create sink element");

    // Create the empty pipeline
    let pipeline = gst::Pipeline::new(Some("video-pipeline"));
    // let pipeline2 = gst::Pipeline::new(Some("audio-pipeline"));

    let mut firststring = String::from(
        r"appsrc ! opusparse ! opusdec !audio/x-raw,rate=24000 ! queue ! audioconvert ! audioresample ! wasapi2sink",
    );
    //let idstring = r"\\\\\?\\SWD\#MMDEVAPI\#\{0.0.0.00000000\}.\{004c4ddc-2fe9-4112-ab8d-29ac78cf4859\}\#\{e6327cad-dcec-4949-ae8a-991e976a79d2\}";
    let secondstring = r"";
    //firststring.push_str(idstring);
    firststring.push_str(secondstring);
    let pipeline_audio_dec = gst::parse_launch(&firststring).unwrap();
    let pipeline_audio = pipeline_audio_dec.dynamic_cast::<gst::Pipeline>().unwrap();

    let qtpaudiosrc = pipeline_audio
        .by_name("appsrc0")
        .expect("cannot set qtpaudiosink");
    // let opusdec = pipeline_audio
    //     .by_name("opusdec0")
    //     .expect("cannot set qtpaudiosink");

    pipeline.add_many(&[&qp2psrc, &decode, &sink]).unwrap();
    gst::Element::link_many(&[&qp2psrc, &decode, &sink]).expect("Elements could not be linked.");
    let public_ip = public_ip::addr_v4()
        .await
        .expect("cannot get public ip address");
    let node = Endpoint::new_client(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 0)),
        Config {
            forward_port: false,

            external_ip: Some(IpAddr::V4(public_ip)),

            idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
            ..Default::default()
        },
    )
    .expect("Cannot create endpoint");
    let node2 = Endpoint::new_client(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 0)),
        Config {
            forward_port: false,

            external_ip: Some(IpAddr::V4(public_ip)),

            idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
            ..Default::default()
        },
    )
    .expect("Cannot create endpoint");
    let node3 = Endpoint::new_client(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 0)),
        Config {
            forward_port: false,

            external_ip: Some(IpAddr::V4(public_ip)),

            idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
            ..Default::default()
        },
    )
    .expect("Cannot create endpoint");
    let node4 = Endpoint::new_client(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 0)),
        Config {
            forward_port: false,

            external_ip: Some(IpAddr::V4(public_ip)),

            idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
            ..Default::default()
        },
    )
    .expect("Cannot create endpoint");

    let peer: SocketAddr = args.ip_connect;
    println!("Trying to connect to {} :", peer);
    let (authconn, mut authincoming) = node4
        .connect_to(&peer)
        .await
        .expect("cannot create connection");
    let mut builder = Builder::default();
    let mut send_input = builder.start_map();
    send_input.push("key", args.key.as_str());
    send_input.push("password", args.password.as_str());
    send_input.end_map();
    // println!(
    //     "Mouse state :{} x :{} y :{}",
    //     *mouse_state_temp, cursor_position_temp.x, cursor_position_temp.y
    // );
    authconn
        .send(Bytes::copy_from_slice(builder.view()))
        .await
        .expect("get error sent input");
    let incominginfo = authincoming.next().await.unwrap();
    let data = incominginfo.expect("cannot get byte");
    let root = Reader::get_root(data.as_byte_slice()).unwrap();
    let read_input = root.as_map();
    let status = read_input.idx("status").as_u32();
    match status {
        0 => {
            let height = read_input.idx("height").as_u32();
            let width = read_input.idx("width").as_u32();
            let framerateget = read_input.idx("framerate").as_u32();
            let downscale = read_input.idx("downscale").as_u32();
            let portvideo = read_input.idx("portvideo").as_u32();
            let portaudio = read_input.idx("portaudio").as_u32();
            let portinput = read_input.idx("portinput").as_u32();
            let event_loop = EventLoop::new();
            let mut monitors: Vec<MonitorHandle> = vec![];
            event_loop.available_monitors().for_each(|handle| {
                println!("Monitor of : {}", &handle.name().unwrap());
                monitors.push(handle);
            });
            println!("get width {} and height {}", width, height);
            // let window = winit::window::Window::new(&event_loop).unwrap();
            let monitorinfo = &monitors[0 as usize];
            let size1 = PhysicalSize {
                height: height, // 864
                width: width,   // 1536
            };
            let framerate: i32 = framerateget as i32;

            let mut size2 = PhysicalSize {
                height: monitorinfo.size().height,
                width: monitorinfo.size().width,
            };
            if downscale == 0 {
                size2.width = size1.width;
                size2.height = size1.height;
            } else {
                let down = 100 - downscale as u32;
                size2.width = (size1.width * 100) / down;
                size2.height = (size1.height * 100) / down;
            }
            let window = winit::window::WindowBuilder::new()
                .with_inner_size(size2)
                // .with_fullscreen(Some(Fullscreen::Borderless(Some(
                //     monitors.get(0).unwrap().clone(),
                // ))))
                .with_title(String::from("Gameview"))
                .build(&event_loop)
                .unwrap();

            let (_connection, incoming_messages) = node
                .connect_to(&SocketAddr::new(peer.ip().clone(), portvideo as u16))
                .await
                .expect("cannot create connection");
            let (_connection2, incoming_messages2) = node2
                .connect_to(&SocketAddr::new(peer.ip().clone(), portaudio as u16))
                .await
                .expect("cannot create connection");
            let (sent_stream_input, _incoming_messages3) = node3
                .connect_to(&SocketAddr::new(peer.ip().clone(), portinput as u16))
                .await
                .expect("cannot create connection");
            let filter_caps2 =
                gst::Caps::builder_full_with_features(CapsFeatures::new(&["memory:D3D11Memory"]))
                    .structure_with_features(
                        Structure::new(
                            "video/x-raw",
                            &[
                                ("format", &"NV12"),
                                ("width", &(size1.width as i32)), //1476
                                ("height", &(size1.height as i32)), //830
                                                                  //("framerate", &gst::Fraction::new(120, 1)),
                            ],
                        ),
                        CapsFeatures::new(&["memory:D3D11Memory"]),
                    )
                    .build();

            let filter_caps3 = gst::Caps::builder_full()
                .structure(Structure::new(
                    "video/x-raw",
                    &[
                        ("format", &"NV12"),
                        ("width", &(size1.width as i32)),   //1476
                        ("height", &(size1.height as i32)), //830
                        ("framerate", &gst::Fraction::new(framerate, 1)),
                    ],
                ))
                .build();

            filter2.set_property("caps", filter_caps2);
            filter3.set_property("caps", filter_caps3);
            //opusdec.set_property("use-inband-fec", true as bool);
            //audiofilter.set_property("caps", audio_filter);
            //audiosink.set_property("low-latency", true as bool);

            let appsrc = qp2psrc
                .dynamic_cast::<gstreamer_app::AppSrc>()
                .expect("Source element is expected to be an appsrc!");
            let appsrc2 = qtpaudiosrc
                .dynamic_cast::<gstreamer_app::AppSrc>()
                .expect("Source element is expected to be an appsrc2!");
            let wgpusink = sink
                .dynamic_cast::<gstreamer_app::AppSink>()
                .expect("Sink element is expected to be an appsink!");

            qtpreceive::qtpreceive(
                framerate,
                downscale,
                size1,
                size2,
                wgpusink,
                window,
                event_loop,
                appsrc,
                pipeline,
                pipeline_audio,
                appsrc2,
                incoming_messages,
                incoming_messages2,
                sent_stream_input,
            )
            .await;
        }
        1 => {
            println!("Username or Passwords is not correct")
        }
        _ => {}
    }
}
