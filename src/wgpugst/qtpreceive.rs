use byte_slice_cast::AsByteSlice;
use bytes::Bytes;
use haphazard::{AtomicPtr, HazardPointer};
use parking_lot::Mutex;
use qp2p::{Config, Connection, ConnectionIncoming, Endpoint, SendStream};
use tokio::{runtime, task, time::Instant};
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
    sync::Arc,
    thread,
    time::Duration,
};

extern crate gstreamer as gst;
extern crate gstreamer_audio as gst_audio;
extern crate gstreamer_base as gst_base;
extern crate gstreamer_video as gst_video;

use gst::{prelude::*, CapsFeatures};

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
mod wgpusurface;
use wgpusurface::Wgpusurface;
mod controls;
use controls::Controls;
mod sendinput;
use sendinput::{send_keyboard, send_mouse};

use flexbuffers;
use flexbuffers::{BitWidth, Builder, Reader, ReaderError};
use winit::monitor::{MonitorHandle, VideoMode};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorIcon, Fullscreen, Window, WindowAttributes},
};

extern crate gstreamer_app;

use std::i32;

use gst::Structure;

use crate::wgpugst::qtpreceive::controls::Message;

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
pub async fn qtpreceive(
    framerate: i32,
    size1: PhysicalSize<u32>,
    size2: PhysicalSize<u32>,
    wgpusink: gstreamer_app::AppSink,
    window: Window,
    event_loop: EventLoop<()>,
    appsrc2: gstreamer_app::AppSrc,
    pipeline: gst::Pipeline,

    pipeline2: gst::Pipeline,
    appsrcaudio: gstreamer_app::AppSrc,
    connect_to: SocketAddr,
) {
    let physical_size = window.inner_size();
    let mut viewport = Viewport::with_physical_size(
        Size::new(physical_size.width, physical_size.height),
        window.scale_factor(),
    );
    // let cursor_position = Arc::new(AtomicPtr::from(Box::new(PhysicalPosition::new(-1.0, -1.0))));
    // let mouse_state = Arc::new(AtomicPtr::from(Box::new(MOUSEEVENTF_MOVE)));
    // let cursor_position_clone = Arc::clone(&cursor_position);
    // let mouse_state_clone = Arc::clone(&mouse_state);
    let mut modifiers = ModifiersState::default();
    let mut clipboard = Clipboard::connect(&window);
    let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);

    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");
    // Create the logical device and command queue
    let swapchain_format = surface
        .get_supported_formats(&adapter)
        .first()
        .copied()
        .expect("Get preferred format");
    // Load the shaders from disk

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size2.width,
        height: size2.height,
        // width: size1.width,
        // height: size1.height,
        present_mode: wgpu::PresentMode::Immediate,
    };
    surface.configure(&device, &config);
    let mut resized = false;

    // Initialize staging belt
    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);

    // Initialize scene and GUI controls

    let mut controls = Controls::new();
    let wgpusurface = Wgpusurface::new(&device, swapchain_format, size1, size2);
    let mut debug = Debug::new();
    let mut renderer = Renderer::new(Backend::new(&device, Settings::default(), swapchain_format));
    let mut state = iced_native::program::State::new(
        controls,
        viewport.logical_size(),
        &mut renderer,
        &mut debug,
    );
    wgpusink.set_caps(Some(
        &gst::Caps::builder_full()
            .structure(Structure::new(
                "video/x-raw",
                &[
                    ("format", &"NV12"),
                    ("width", &(size1.width as i32)), //1476
                    ("height", &(size1.height as i32)), //830
                                                      //("framerate", &gst::Fraction::new(framerate, 1)),
                ],
            ))
            .build(),
    ));

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
    // println!("check for reachable ...");
    // node.is_reachable(&SocketAddr::from((Ipv4Addr::new(60, 52, 227, 195), 56683)))
    //     .await
    //     .expect("cannot reach node");
    println!("created endpoint");
    appsrc2.set_caps(Some(
        &gst::Caps::builder_full()
            .structure(Structure::new(
                "video/x-h265",
                &[
                    ("stream-format", &"byte-stream"),
                    ("alignment", &"au"),
                    ("profile", &"main"),
                    ("width", &(size1.width as i32)),   //1476
                    ("height", &(size1.height as i32)), //830
                    // ("framerate", &gst::Fraction::new(60, 1)),
                    ("chroma-site", &"mpeg2"),
                    ("colorimetry", &"bt709"),
                ],
            ))
            .build(),
    ));
    appsrcaudio.set_caps(Some(
        &gst::Caps::builder_full()
            .structure(Structure::new("audio/x-opus", &[]))
            .build(),
    ));

    let peer: SocketAddr = connect_to;
    println!("Trying to connect to {} :", peer);
    let (authconn, mut authincoming) = node4
        .connect_to(&peer)
        .await
        .expect("cannot create connection");
    let mut builder = Builder::default();
    let mut send_input = builder.start_map();
    send_input.push("key", "hello");
    send_input.push("password", "123");
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
            let portvideo = read_input.idx("portvideo").as_u32();
            let portaudio = read_input.idx("portaudio").as_u32();
            let portinput = read_input.idx("portinput").as_u32();

            // size1.height = height as u32;
            // size1.width = width as u32;

            let (connection, mut incoming_messages) = node
                .connect_to(&SocketAddr::new(peer.ip().clone(), portvideo as u16))
                .await
                .expect("cannot create connection");
            let (_connection2, mut incoming_messages2) = node2
                .connect_to(&SocketAddr::new(peer.ip().clone(), portaudio as u16))
                .await
                .expect("cannot create connection");
            let (sent_stream_input, _incoming_messages3) = node3
                .connect_to(&SocketAddr::new(peer.ip().clone(), portinput as u16))
                .await
                .expect("cannot create connection");

            //let _src = connection.remote_address();

            // let windowarc = Arc::new(window);
            // let windowclone = Arc::clone(&windowarc);
            // let windowclone2 = Arc::clone(&windowarc);
            // let wgpusurfacearc = Arc::new(wgpusurface);
            // let wgpusurfaceclone = Arc::clone(&wgpusurfacearc);
            // let wgpusurfaceclone2 = Arc::clone(&wgpusurfacearc);
            let queuearc = Arc::new(queue);
            let queueclone = Arc::clone(&queuearc);
            let queueclone2 = Arc::clone(&queuearc);

            // let indicator = Arc::new(AtomicPtr::from(Box::new(0 as i8)));
            // let send_indicator_clone = Arc::clone(&indicator);
            // let send_indicator_clone2 = Arc::clone(&indicator);

            appsrc2.set_callbacks(
                gstreamer_app::AppSrcCallbacks::builder()
                    .need_data(move |appsrc3, _| {
                        pollster::block_on(async {
                            //let now2 = Instant::now();
                            let bytes = incoming_messages.next().await.expect("cannot get bytes");

                            //println!("Received info with took {} us.", now2.elapsed().as_micros());
                            if bytes.clone() != None {
                                match appsrc3
                                    .push_buffer(gst::Buffer::from_slice(bytes.expect("msg")))
                                {
                                    Ok(_data) => {}
                                    Err(_err) => {}
                                }
                            }
                        });
                    })
                    .build(),
            );

            appsrcaudio.set_callbacks(
                gstreamer_app::AppSrcCallbacks::builder()
                    .need_data(move |appsrcaudio2, _| {
                        pollster::block_on(async {
                            //   let now = Instant::now();

                            //let now2 = Instant::now();
                            let bytes = incoming_messages2.next().await.expect("cannot get bytes");

                            if bytes.clone() != None {
                                match appsrcaudio2
                                    .push_buffer(gst::Buffer::from_slice(bytes.expect("msg")))
                                {
                                    Ok(_data) => {}
                                    Err(_err) => {}
                                }
                                // vecclone2
                                //     .push()
                                //     .expect("cannot push buffer");
                            }
                            // if vecclone.len() > 0 {

                            // }

                            //tokio::time::sleep(Duration::from_nanos(200)).await;
                            //println!("Received info with took {} us.", now2.elapsed().as_micros());
                        });
                    })
                    .build(),
            );

            let surfacearc = Arc::new(surface);
            let surfaceclone = Arc::clone(&surfacearc);
            let surfaceclone2 = Arc::clone(&surfacearc);
            let wgpusurfacearc = Arc::new(wgpusurface);
            let wgpusurfaceclone = Arc::clone(&wgpusurfacearc);
            let wgpusurfaceclone2 = Arc::clone(&wgpusurfacearc);

            // let value1 = 1 as u8;
            // let testarc = Arc::new(AtomicPtr::new(value1));

            wgpusink.set_callbacks(
                gstreamer_app::AppSinkCallbacks::builder()
                    // Add a handler to the "new-sample" signal.
                    .new_sample(move |appsink| {
                        // Pull the sample in question out of the appsink's buffer.
                        match appsink.pull_sample().map_err(|_| gst::FlowError::Eos) {
                            Ok(sample) => {
                                let caps = sample.caps().expect("cannot get caps");
                                match gst_video::VideoInfo::from_caps(caps) {
                                    Ok(video_info) => {
                                        let buffer =
                                            sample.buffer_owned().expect("cannot get buffer");
                                        let frame = gst_video::VideoFrame::from_buffer_readable(
                                            buffer.clone(),
                                            &video_info,
                                        )
                                        .map_err(|_| gst::FlowError::Error)?;

                                        wgpusurfaceclone.write_texture(
                                            &queueclone,
                                            size1,
                                            frame.plane_data(0).expect("cannot get plane data"),
                                            frame.plane_data(1).expect("cannot get plane data"),
                                        );

                                        // let mut ind = HazardPointer::new();
                                        // let send_indicator_temp = render_or_notclone.safe_load(&mut ind).expect("msg");
                                    }
                                    Err(err) => println!("video info cannot be get {}", err),
                                }
                            }
                            Err(err) => println!("sample cannot be input {}", err),
                        };

                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );

            pipeline
                .set_state(gst::State::Playing)
                .expect("cannot set ready");
            pipeline2
                .set_state(gst::State::Playing)
                .expect("cannot set ready");

            let sent_stream_input_arc = Arc::new(sent_stream_input);

            let cursor_position =
                Arc::new(AtomicPtr::from(Box::new(PhysicalPosition::new(0.0, 0.0))));

            // let mut last_key = 0 as u32;
            let mut a: i32 = 0;
            let mut total: i128 = 0;
            let mut current_latency = 0;
            let threaded_rt = runtime::Runtime::new().unwrap();
            event_loop.run(move |event, _, control_flow| {
                // You should change this if you want to render continuosly
                // let timenow = Instant::now();
                if total >= 1000000 {
                    current_latency = a;
                    total = 0;
                    a = 0;
                }
                let now = Instant::now();
                // *control_flow = ControlFlow::Wait;
                let sent_stream_input_clone = Arc::clone(&sent_stream_input_arc);
                let cursor_position_clone = Arc::clone(&cursor_position);

                match event {
                    Event::WindowEvent { event, .. } => {
                        match event {
                            WindowEvent::CursorMoved { mut position, .. } => {
                                // cursor_position_clone.swap(Box::new(position));
                                // mouse_state_clone.swap(Box::new(MOUSEEVENTF_MOVE));
                                // send_indicator_clone2.swap(Box::new(1));
                                if size1.height != size2.height || size1.width != size2.width {
                                    let percnheight = size1.height as f64 / size2.height as f64;
                                    let percnwidth = size1.width as f64 / size2.width as f64;
                                    position.x = percnwidth * position.x;
                                    position.y = percnheight * position.y;
                                    cursor_position.swap(Box::new(position));
                                    threaded_rt.spawn(async move {
                                        send_mouse(
                                            MOUSEEVENTF_MOVE,
                                            &position,
                                            &sent_stream_input_clone,
                                            0,
                                        )
                                        .await;
                                    });
                                } else {
                                    cursor_position.swap(Box::new(position));
                                    threaded_rt.spawn(async move {
                                        send_mouse(
                                            MOUSEEVENTF_MOVE,
                                            &position,
                                            &sent_stream_input_clone,
                                            0,
                                        )
                                        .await;
                                    });
                                }
                            }
                            WindowEvent::ModifiersChanged(new_modifiers) => {
                                modifiers = new_modifiers;
                            }

                            WindowEvent::Resized(new_size) => {
                                viewport = Viewport::with_physical_size(
                                    Size::new(new_size.width, new_size.height),
                                    window.scale_factor(),
                                );

                                resized = true;
                            }
                            WindowEvent::CloseRequested => {
                                *control_flow = ControlFlow::Exit;
                            }
                            WindowEvent::CursorEntered { .. } => {
                                //window.set_cursor_visible(false);
                            }
                            WindowEvent::CursorLeft { .. } => {
                                // window.set_cursor_visible(true);
                            }
                            WindowEvent::Focused(focused) => {
                                // println!("Scan code :{}", last_key);
                                // if !focused {
                                //     tokio::spawn(async move {
                                //         send_keyboard(
                                //             KEYEVENTF_KEYUP as u16,
                                //             last_key as u16,
                                //             &sent_stream_input_clone,
                                //         )
                                //         .await;
                                //     });
                                // }
                            }
                            WindowEvent::KeyboardInput { input, .. } => {
                                match input.state {
                                    winit::event::ElementState::Pressed => {
                                        // if last_key != input.scancode {
                                        // last_key = input.scancode;

                                        threaded_rt.spawn(async move {
                                            send_keyboard(
                                                KEYEVENTF_SCANCODE as u16,
                                                input.scancode as u16,
                                                &sent_stream_input_clone,
                                            )
                                            .await;
                                        });
                                        // }
                                        // println!("Keybooard press :{}",);
                                    }
                                    winit::event::ElementState::Released => {
                                        threaded_rt.spawn(async move {
                                            send_keyboard(
                                                (KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP) as u16,
                                                input.scancode as u16,
                                                &sent_stream_input_clone,
                                            )
                                            .await;
                                        });
                                    }
                                }
                            }
                            WindowEvent::MouseWheel { delta, .. } => match delta {
                                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                                    println!("Mouse scroll x:{} y:{}", x, y);
                                    threaded_rt.spawn(async move {
                                        let mut ind = HazardPointer::new();
                                        let cursor_position_temp =
                                            cursor_position_clone.safe_load(&mut ind).expect("msg");
                                        send_mouse(
                                            MOUSEEVENTF_WHEEL,
                                            cursor_position_temp,
                                            &sent_stream_input_clone,
                                            y as u32,
                                        )
                                        .await;
                                    });
                                }
                                winit::event::MouseScrollDelta::PixelDelta(x) => {}
                            },
                            WindowEvent::MouseInput { state, button, .. } => match state {
                                winit::event::ElementState::Pressed => match button {
                                    winit::event::MouseButton::Left => {
                                        // println!("Left button press down");

                                        // mouse_state_clone.swap(Box::new(MOUSEEVENTF_LEFTDOWN));
                                        // send_indicator_clone2.swap(Box::new(1));

                                        threaded_rt.spawn(async move {
                                            let mut ind = HazardPointer::new();
                                            let cursor_position_temp = cursor_position_clone
                                                .safe_load(&mut ind)
                                                .expect("msg");
                                            send_mouse(
                                                MOUSEEVENTF_LEFTDOWN,
                                                cursor_position_temp,
                                                &sent_stream_input_clone,
                                                0,
                                            )
                                            .await;
                                        });
                                    }
                                    winit::event::MouseButton::Right => {
                                        // println!("Right button press  down");

                                        // mouse_state_clone.swap(Box::new(MOUSEEVENTF_RIGHTDOWN));
                                        // send_indicator_clone2.swap(Box::new(1));

                                        threaded_rt.spawn(async move {
                                            let mut ind = HazardPointer::new();
                                            let cursor_position_temp = cursor_position_clone
                                                .safe_load(&mut ind)
                                                .expect("msg");
                                            send_mouse(
                                                MOUSEEVENTF_RIGHTDOWN,
                                                cursor_position_temp,
                                                &sent_stream_input_clone,
                                                0,
                                            )
                                            .await;
                                        });
                                    }
                                    winit::event::MouseButton::Middle => {
                                        // println!("Middle button press  down");

                                        // mouse_state_clone.swap(Box::new(MOUSEEVENTF_MIDDLEDOWN));
                                        // send_indicator_clone2.swap(Box::new(1));

                                        threaded_rt.spawn(async move {
                                            let mut ind = HazardPointer::new();
                                            let cursor_position_temp = cursor_position_clone
                                                .safe_load(&mut ind)
                                                .expect("msg");
                                            send_mouse(
                                                MOUSEEVENTF_MIDDLEDOWN,
                                                cursor_position_temp,
                                                &sent_stream_input_clone,
                                                0,
                                            )
                                            .await;
                                        });
                                    }
                                    winit::event::MouseButton::Other(num) => {
                                        println!("Other button press down {}", num);
                                    }
                                },
                                winit::event::ElementState::Released => match button {
                                    winit::event::MouseButton::Left => {
                                        // println!("Left button press up");

                                        // mouse_state_clone.swap(Box::new(MOUSEEVENTF_LEFTUP));
                                        // send_indicator_clone2.swap(Box::new(1));

                                        threaded_rt.spawn(async move {
                                            let mut ind = HazardPointer::new();
                                            let cursor_position_temp = cursor_position_clone
                                                .safe_load(&mut ind)
                                                .expect("msg");
                                            send_mouse(
                                                MOUSEEVENTF_LEFTUP,
                                                cursor_position_temp,
                                                &sent_stream_input_clone,
                                                0,
                                            )
                                            .await;
                                        });
                                    }
                                    winit::event::MouseButton::Right => {
                                        // println!("Right button press  up");

                                        // mouse_state_clone.swap(Box::new(MOUSEEVENTF_RIGHTUP));
                                        // send_indicator_clone2.swap(Box::new(1));

                                        threaded_rt.spawn(async move {
                                            let mut ind = HazardPointer::new();
                                            let cursor_position_temp = cursor_position_clone
                                                .safe_load(&mut ind)
                                                .expect("msg");
                                            send_mouse(
                                                MOUSEEVENTF_RIGHTUP,
                                                cursor_position_temp,
                                                &sent_stream_input_clone,
                                                0,
                                            )
                                            .await;
                                        });
                                    }
                                    winit::event::MouseButton::Middle => {
                                        // println!("Middle button press  up");

                                        // mouse_state_clone.swap(Box::new(MOUSEEVENTF_MIDDLEUP));
                                        // send_indicator_clone2.swap(Box::new(1));

                                        threaded_rt.spawn(async move {
                                            let mut ind = HazardPointer::new();
                                            let cursor_position_temp = cursor_position_clone
                                                .safe_load(&mut ind)
                                                .expect("msg");
                                            send_mouse(
                                                MOUSEEVENTF_MIDDLEUP,
                                                cursor_position_temp,
                                                &sent_stream_input_clone,
                                                0,
                                            )
                                            .await;
                                        });
                                    }
                                    winit::event::MouseButton::Other(num) => {
                                        println!("Other button press up {}", num);
                                    }
                                },
                            },
                            _ => {}
                        }

                        // Map window event to iced event
                        if let Some(event1) = iced_winit::conversion::window_event(
                            &event,
                            window.scale_factor(),
                            modifiers,
                        ) {
                            //let lockarc = eventarcclone.lock();
                            state.queue_event(event1);
                        }
                    }
                    Event::MainEventsCleared => {
                        // If there are events pending
                        window.request_redraw();
                        if !state.is_queue_empty() {
                            // let mut h = HazardPointer::new();
                            // let cursor_position_temp =
                            //     cursor_position_clone.safe_load(&mut h).expect("msg");
                            // We update iced
                            let mut ind = HazardPointer::new();
                            let cursor_position_temp =
                                cursor_position_clone.safe_load(&mut ind).expect("msg");
                            let _ = state.update(
                                viewport.logical_size(),
                                conversion::cursor_position(
                                    cursor_position_temp.cast(),
                                    viewport.scale_factor(),
                                ),
                                &mut renderer,
                                &iced_wgpu::Theme::Dark,
                                &renderer::Style {
                                    text_color: Color::WHITE,
                                },
                                &mut clipboard,
                                &mut debug,
                            );

                            // and request a redraw
                        }
                    }
                    Event::RedrawRequested(_) => {
                        if resized {
                            let size = window.inner_size();

                            surfaceclone2.configure(
                                &device,
                                &wgpu::SurfaceConfiguration {
                                    format: swapchain_format,
                                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                    width: size.width,
                                    height: size.height,
                                    present_mode: wgpu::PresentMode::Immediate,
                                },
                            );

                            resized = false;
                        }
                        match surfaceclone.get_current_texture() {
                            Ok(frame) => {
                                let view_texture = &frame.texture;
                                let view = view_texture
                                    .create_view(&wgpu::TextureViewDescriptor::default());
                                //surface_texture_view_clone.swap(Box::new(view));
                                // let mut ind2 = HazardPointer::new();
                                // let surface_texture_view_clone_temp = surface_texture_view_clone
                                //     .safe_load(&mut ind2)
                                //     .expect("msg");
                                let mut encoder = device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor { label: None },
                                );
                                // let program = state.program();

                                if current_latency != 0 {
                                    state.queue_message(Message::TextChanged(
                                        current_latency.to_string(),
                                    ));
                                }
                                // {
                                //     let mut rpass = wgpusurfaceclone2.yuv_renderpass(&mut encoder);
                                //     wgpusurfaceclone2.yuv_draw(&mut rpass);
                                // }
                                {
                                    let mut cpass = wgpusurfaceclone2.yuv_computepass(&mut encoder);
                                    wgpusurfaceclone2.yuv_compute(size1, &mut cpass);
                                }
                                {
                                    wgpusurfaceclone2.transfertexture(&mut encoder, size1);
                                }
                                {
                                    let mut rpass = wgpusurfaceclone2.easu_renderpass(&mut encoder);
                                    wgpusurfaceclone2.easu_draw(&mut rpass);
                                }
                                {
                                    let mut rpass = wgpusurfaceclone2.lcas_renderpass(&mut encoder);
                                    wgpusurfaceclone2.lcas_draw(&mut rpass);
                                }
                                {
                                    let mut rpass =
                                        wgpusurfaceclone2.rcas_renderpass(&mut encoder, &view);
                                    wgpusurfaceclone2.rcas_draw(&mut rpass);
                                }

                                renderer.with_primitives(|backend, primitive| {
                                    backend.present(
                                        &device,
                                        &mut staging_belt,
                                        &mut encoder,
                                        &view,
                                        primitive,
                                        &viewport,
                                        &debug.overlay(),
                                    );
                                });

                                // Then we submit the work
                                staging_belt.finish();
                                // Update the mouse cursor
                                window.set_cursor_icon(iced_winit::conversion::mouse_interaction(
                                    state.mouse_interaction(),
                                ));

                                queueclone2.submit(Some(encoder.finish()));
                                frame.present();
                                staging_belt.recall();
                                // And recall staging buffers

                                total = total + now.elapsed().as_micros() as i128;
                                a = a + 1;
                            }
                            Err(error) => match error {
                                wgpu::SurfaceError::OutOfMemory => {
                                    panic!("Swapchain error: {}. Rendering cannot continue.", error)
                                }
                                _ => {
                                    // Try rendering again next frame.
                                    //windowclone2.request_redraw();
                                }
                            },
                        }
                    }

                    _ => {}
                }
            });
        }
        1 => {
            println!("Username or Passwords is not correct")
        }
        _ => {}
    }

    // node.close();
}
