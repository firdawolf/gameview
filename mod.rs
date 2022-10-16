use bytes::Bytes;
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

use std::i32;

use gst::Structure;

//all public function

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

pub fn sent(bitrate: u32, shutdown_rx: Receiver<()>) {
    // Initialize GStreamer
    gst::init().unwrap();

    // gst-inspect-1.0.exe mediafoundation
    // Create the elements

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

    // let sink = gst::ElementFactory::make("d3d11videosink", Some("sink"))
    //     .expect("Could not create sink element");
    // let sink =
    //     gst::ElementFactory::make("rswgpu", Some("sink")).expect("Could not create sink element");

    let size1 = PhysicalSize {
        height: 864, // 864
        width: 1536, // 1536
    };
    let framerate: i32 = 75;
    // let size2 = PhysicalSize {
    //     height: 1080,
    //     width: 1920,
    // };

    // Create the empty pipeline
    let pipeline = gst::Pipeline::new(Some("video-pipeline"));
    // let pipeline2 = gst::Pipeline::new(Some("audio-pipeline"));
    let mut firststring = String::from(r"wasapi2src device=");
    let idstring = &device();
    let secondstring = r" loopback=true ! audioresample ! audioconvert ! queue ! audio/x-raw,rate=48000 ! opusenc ! appsink";
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
    source.set_property("show-cursor", false as bool);
    encode.set_property("bframes", 1 as u32);
    encode.set_property("max-bitrate", bitrate as u32);
    encode.set_property("bitrate", bitrate as u32);
    encode.set_property("quality-vs-speed", 50 as u32);
    encode.set_property_from_str("rc-mode", "cbr");
    // encode.set_property("min-qp", 10 as u32);
    // encode.set_property("max-qp", 15 as u32);
    //encode.set_property("qp", 15 as u32);

    encode.set_property("low-latency", false as bool);
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
    opusenc.set_property("bitrate", 128000 as i32);
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
        shutdown_rx,
    );
    // tokio::runtime::Builder::new_current_thread()
    //     .enable_all()
    //     .build()
    //     .unwrap()
    //     .block_on(async {
    //         qtpsend(source, qtp2sink, pipeline, size1).await;
    //     });
}

pub fn receive(connect_to: SocketAddr) {
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
    // let sink = gst::ElementFactory::make("d3d11videosink", Some("sink"))
    //     .expect("Could not create sink element");
    // let sink =
    //     gst::ElementFactory::make("rswgpu", Some("sink")).expect("Could not create sink element");

    let size1 = PhysicalSize {
        height: 864, // 864
        width: 1536, // 1536
    };
    let framerate: i32 = 75;
    let size2 = PhysicalSize {
        height: 1080,
        width: 1920,
    };
    // Create the empty pipeline
    let pipeline = gst::Pipeline::new(Some("video-pipeline"));
    // let pipeline2 = gst::Pipeline::new(Some("audio-pipeline"));

    let mut firststring = String::from(
        r"appsrc ! opusparse ! opusdec !audio/x-raw,rate=48000 ! queue ! audioconvert ! audioresample ! wasapi2sink",
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
    // let audio_filter = &gst::Caps::builder_full()
    //     .structure(Structure::new(
    //         "audio/x-raw",
    //         &[
    //             ("format", &"S16LE"),
    //             ("layout", &"interleaved"),
    //             ("rate", &(48000 as i32)), //1476
    //             ("channels", &(2 as i32)), //830
    //         ],
    //     ))
    //     .build();
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

    //queue.set_property("min-threshold-time", 500000 as u64);
    let appsrc = qp2psrc
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc!");
    let appsrc2 = qtpaudiosrc
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc2!");
    let wgpusink = sink
        .dynamic_cast::<gstreamer_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");
    let event_loop = EventLoop::new();
    let mut monitors: Vec<MonitorHandle> = vec![];
    event_loop.available_monitors().for_each(|handle| {
        println!("Monitor of : {}", &handle.name().unwrap());
        monitors.push(handle);
    });
    // let window = winit::window::Window::new(&event_loop).unwrap();

    let window = winit::window::WindowBuilder::new()
        .with_inner_size(size2)
        // .with_fullscreen(Some(Fullscreen::Borderless(Some(
        //     monitors.get(0).unwrap().clone(),
        // ))))
        .with_title(String::from("Wgpu Render"))
        .build(&event_loop)
        .unwrap();
    qtpreceive::qtpreceive(
        framerate,
        size1,
        size2,
        wgpusink,
        window,
        event_loop,
        appsrc,
        pipeline,
        pipeline_audio,
        appsrc2,
        connect_to,
    );
}

// #[tokio::main]
// pub async fn menu() {
//     let event_loop = EventLoop::new();
//     let size1 = PhysicalSize {
//         height: 800, // 864
//         width: 800,  // 1536
//     };
//     let mut monitors: Vec<MonitorHandle> = vec![];
//     event_loop.available_monitors().for_each(|handle| {
//         println!("Monitor of : {}", &handle.name().unwrap());
//         monitors.push(handle);
//     });
//     let mut title = "";
//     // let window = winit::window::Window::new(&event_loop).unwrap();

//     let window = winit::window::WindowBuilder::new()
//         .with_inner_size(size1)
//         // .with_fullscreen(Some(Fullscreen::Borderless(Some(
//         //     monitors.get(0).unwrap().clone(),
//         // ))))
//         .with_title(String::from(title))
//         .build(&event_loop)
//         .unwrap();
//     let physical_size = window.inner_size();
//     let mut viewport = Viewport::with_physical_size(
//         Size::new(physical_size.width, physical_size.height),
//         window.scale_factor(),
//     );
//     // let cursor_position = Arc::new(AtomicPtr::from(Box::new(PhysicalPosition::new(-1.0, -1.0))));
//     // let mouse_state = Arc::new(AtomicPtr::from(Box::new(MOUSEEVENTF_MOVE)));
//     // let cursor_position_clone = Arc::clone(&cursor_position);
//     // let mouse_state_clone = Arc::clone(&mouse_state);
//     let mut modifiers = ModifiersState::default();
//     let mut clipboard = Clipboard::connect(&window);
//     let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);

//     let surface = unsafe { instance.create_surface(&window) };
//     let adapter = instance
//         .request_adapter(&wgpu::RequestAdapterOptions {
//             power_preference: wgpu::PowerPreference::default(),
//             force_fallback_adapter: false,
//             // Request an adapter which can render to our surface
//             compatible_surface: Some(&surface),
//         })
//         .await
//         .expect("Failed to find an appropriate adapter");
//     let (device, queue) = adapter
//         .request_device(
//             &wgpu::DeviceDescriptor {
//                 label: None,
//                 features: wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
//                 // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
//                 limits: wgpu::Limits::downlevel_webgl2_defaults()
//                     .using_resolution(adapter.limits()),
//             },
//             None,
//         )
//         .await
//         .expect("Failed to create device");
//     // Create the logical device and command queue
//     let swapchain_format = surface
//         .get_supported_formats(&adapter)
//         .first()
//         .copied()
//         .expect("Get preferred format");
//     // Load the shaders from disk

//     let config = wgpu::SurfaceConfiguration {
//         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
//         format: swapchain_format,
//         width: size1.width,
//         height: size1.height,
//         // width: size1.width,
//         // height: size1.height,
//         present_mode: wgpu::PresentMode::Immediate,
//     };
//     surface.configure(&device, &config);
//     let mut resized = false;

//     // Initialize staging belt
//     let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);

//     // Initialize scene and GUI controls

//     let mut controls = Gameview::new();
//     let wgpusurface = Wgpusurface::new(&device, swapchain_format, size1, size2);
//     let mut debug = Debug::new();
//     let mut renderer = Renderer::new(Backend::new(&device, Settings::default(), swapchain_format));
//     let mut state = iced_native::program::State::new(
//         controls,
//         viewport.logical_size(),
//         &mut renderer,
//         &mut debug,
//     );
//     event_loop.run(move |event, _, control_flow| {
//         // You should change this if you want to render continuosly
//         // let timenow = Instant::now();
//         if total >= 1000000 {
//             current_latency = a;
//             total = 0;
//             a = 0;
//         }
//         let now = Instant::now();
//         // *control_flow = ControlFlow::Wait;
//         let sent_stream_input_clone = Arc::clone(&sent_stream_input_arc);
//         let cursor_position_clone = Arc::clone(&cursor_position);

//         match event {
//             Event::WindowEvent { event, .. } => {
//                 match event {
//                     WindowEvent::CursorMoved { position, .. } => {}
//                     WindowEvent::ModifiersChanged(new_modifiers) => {
//                         modifiers = new_modifiers;
//                     }

//                     WindowEvent::Resized(new_size) => {
//                         viewport = Viewport::with_physical_size(
//                             Size::new(new_size.width, new_size.height),
//                             window.scale_factor(),
//                         );

//                         resized = true;
//                     }
//                     WindowEvent::CloseRequested => {
//                         *control_flow = ControlFlow::Exit;
//                     }
//                     WindowEvent::CursorEntered { .. } => {}
//                     WindowEvent::CursorLeft { .. } => {}
//                     WindowEvent::Focused(focused) => {
//                         // println!("Scan code :{}", last_key);
//                         // if !focused {
//                         //     tokio::spawn(async move {
//                         //         send_keyboard(
//                         //             KEYEVENTF_KEYUP as u16,
//                         //             last_key as u16,
//                         //             &sent_stream_input_clone,
//                         //         )
//                         //         .await;
//                         //     });
//                         // }
//                     }
//                     WindowEvent::KeyboardInput { input, .. } => {}
//                     WindowEvent::MouseWheel { delta, .. } => match delta {},
//                     WindowEvent::MouseInput { state, button, .. } => match state {},
//                     _ => {}
//                 }

//                 // Map window event to iced event
//                 if let Some(event1) =
//                     iced_winit::conversion::window_event(&event, window.scale_factor(), modifiers)
//                 {
//                     //let lockarc = eventarcclone.lock();
//                     state.queue_event(event1);
//                 }
//             }
//             Event::MainEventsCleared => {
//                 // If there are events pending
//                 window.request_redraw();
//                 if !state.is_queue_empty() {
//                     // let mut h = HazardPointer::new();
//                     // let cursor_position_temp =
//                     //     cursor_position_clone.safe_load(&mut h).expect("msg");
//                     // We update iced
//                     let mut ind = HazardPointer::new();
//                     let cursor_position_temp =
//                         cursor_position_clone.safe_load(&mut ind).expect("msg");
//                     let _ = state.update(
//                         viewport.logical_size(),
//                         conversion::cursor_position(
//                             cursor_position_temp.cast(),
//                             viewport.scale_factor(),
//                         ),
//                         &mut renderer,
//                         &iced_wgpu::Theme::Dark,
//                         &renderer::Style {
//                             text_color: Color::WHITE,
//                         },
//                         &mut clipboard,
//                         &mut debug,
//                     );

//                     // and request a redraw
//                 }
//             }
//             Event::RedrawRequested(_) => {
//                 if resized {
//                     let size = window.inner_size();

//                     surfaceclone2.configure(
//                         &device,
//                         &wgpu::SurfaceConfiguration {
//                             format: swapchain_format,
//                             usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
//                             width: size.width,
//                             height: size.height,
//                             present_mode: wgpu::PresentMode::Immediate,
//                         },
//                     );

//                     resized = false;
//                 }
//                 match surfaceclone.get_current_texture() {
//                     Ok(frame) => {
//                         let view_texture = &frame.texture;
//                         let view =
//                             view_texture.create_view(&wgpu::TextureViewDescriptor::default());
//                         //surface_texture_view_clone.swap(Box::new(view));
//                         // let mut ind2 = HazardPointer::new();
//                         // let surface_texture_view_clone_temp = surface_texture_view_clone
//                         //     .safe_load(&mut ind2)
//                         //     .expect("msg");
//                         let mut encoder =
//                             device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
//                                 label: None,
//                             });
//                         // let program = state.program();

//                         if current_latency != 0 {
//                             state.queue_message(Message::TextChanged(current_latency.to_string()));
//                         }
//                         {
//                             let mut rpass = wgpusurfaceclone2.yuv_renderpass(&mut encoder);
//                             wgpusurfaceclone2.yuv_draw(&mut rpass);
//                         }
//                         {
//                             let mut rpass = wgpusurfaceclone2.easu_renderpass(&mut encoder);
//                             wgpusurfaceclone2.easu_draw(&mut rpass);
//                         }
//                         {
//                             let mut rpass = wgpusurfaceclone2.lcas_renderpass(&mut encoder);
//                             wgpusurfaceclone2.lcas_draw(&mut rpass);
//                         }
//                         {
//                             let mut rpass = wgpusurfaceclone2.rcas_renderpass(&mut encoder, &view);
//                             wgpusurfaceclone2.rcas_draw(&mut rpass);
//                         }

//                         renderer.with_primitives(|backend, primitive| {
//                             backend.present(
//                                 &device,
//                                 &mut staging_belt,
//                                 &mut encoder,
//                                 &view,
//                                 primitive,
//                                 &viewport,
//                                 &debug.overlay(),
//                             );
//                         });

//                         // Then we submit the work
//                         staging_belt.finish();
//                         // Update the mouse cursor
//                         window.set_cursor_icon(iced_winit::conversion::mouse_interaction(
//                             state.mouse_interaction(),
//                         ));

//                         queueclone2.submit(Some(encoder.finish()));
//                         frame.present();
//                         staging_belt.recall();
//                         // And recall staging buffers

//                         total = total + now.elapsed().as_micros() as i128;
//                         a = a + 1;
//                     }
//                     Err(error) => match error {
//                         wgpu::SurfaceError::OutOfMemory => {
//                             panic!("Swapchain error: {}. Rendering cannot continue.", error)
//                         }
//                         _ => {
//                             // Try rendering again next frame.
//                             //windowclone2.request_redraw();
//                         }
//                     },
//                 }
//             }
//             _ => {}
//         }
//     });
// }
