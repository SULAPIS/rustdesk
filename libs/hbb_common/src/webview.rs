use reqwest::ResponseBuilderExt;
use std::ops::Deref;
use std::{collections::HashMap, sync::mpsc::Receiver};
use wry::{
    application::{
        event::{DeviceEvent, ElementState, Event, KeyEvent, RawKeyEvent, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        keyboard::KeyCode,
        platform::windows::EventLoopExtWindows,
        window::{Fullscreen, Window, WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

#[derive(Clone)]
enum UserEvents {
    CloseWindow(WindowId),
    NewWindow(),
}

pub fn spawn_webview(rx: Receiver<(i32, i32, i32, i32, i32, String)>) {
    std::thread::spawn(move || start(rx));
}

fn create_new_window(
    title: String,
    event_loop: &EventLoopWindowTarget<UserEvents>,
    proxy: EventLoopProxy<UserEvents>,
    k: usize,
    url: &str,
) -> (WindowId, WebView) {
    let window = WindowBuilder::new()
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top(true)
        .with_visible(false)
        .build(event_loop)
        .unwrap();
    let window_id = window.id();
    let handler = move |window: &Window, req: String| match req.as_str() {
        "new-window" => {
            let _ = proxy.send_event(UserEvents::NewWindow());
        }
        "close" => {
            let _ = proxy.send_event(UserEvents::CloseWindow(window.id()));
        }
        _ if req.starts_with("change-title") => {
            let title = req.replace("change-title:", "");
        }
        _ => {}
    };

    let webview;
    webview = WebViewBuilder::new(window)
        .unwrap()
        .with_url(url)
        .unwrap()
        .with_ipc_handler(handler)
        .build()
        .unwrap();

    (window_id, webview)
}

// struct Webviews{
//     webview:HashMap<WindowId,WebView>,
//     num
// }

#[tokio::main(flavor = "multi_thread")]
async fn start(rx: Receiver<(i32, i32, i32, i32, i32, String)>) {
    let event_loop = EventLoop::<UserEvents>::new_any_thread();
    let mut visible_win_id = -1;
    let mut now_window: Option<(WindowId, WebView)> = None;
    let mut x1 = 0;
    let mut y1 = 0;
    let mut x2 = 0;
    let mut y2 = 0;

    // let mut webviews = HashMap::new();
    let mut map: HashMap<i32, (WindowId, WebView)> = HashMap::new();
    let proxy = event_loop.create_proxy();

    event_loop.run(move |event, event_loop, control_flow| {
        let rec = rx.try_recv();
        match rec {
            Ok((event, x, y, w, h, url)) => match event {
                0 => {
                    if visible_win_id != -1 {
                        let window = map.get(&visible_win_id).unwrap().1.window();
                        let mut pos = window.outer_position().unwrap();
                        pos.x = x + x1; //宽
                        pos.y = y + y1;
                        window.set_outer_position(pos);
                        let mut size = window.inner_size();
                        size.width = (w - x1 - x2) as _;
                        size.height = (h - y1 - y2) as _;

                        window.set_inner_size(size);
                    }
                }
                1 => {
                    println!("event 1");
                    let new_window = create_new_window(
                        format!("Window {}", 0 + 1),
                        &event_loop,
                        proxy.clone(),
                        0,
                        &url,
                    );
                    map.insert(x, new_window);
                }
                2 => {
                    println!("event 2");
                    if visible_win_id != -1 {
                        let window = map.get(&visible_win_id).unwrap().1.window();
                        window.set_visible(false);
                    }
                    println!("get {}", x);
                    let window = map.get(&x);

                    match window {
                        Some(w) => {
                            let window = w.1.window();
                            window.set_visible(true);
                            visible_win_id = x;
                        }
                        None => {}
                    }
                }
                3 => {
                    println!("event 3");
                    if visible_win_id != -1 {
                        let window = map.get(&visible_win_id).unwrap().1.window();
                        window.set_visible(false);
                        visible_win_id = -1;
                    }
                }
                4 => {
                    println!("event 4");
                    if url != "" {
                        let new_window = create_new_window(
                            format!("Window {}", 0 + 1),
                            &event_loop,
                            proxy.clone(),
                            0,
                            &url,
                        );

                        new_window.1.window().set_maximized(true);
                        new_window.1.window().set_visible(true);
                        now_window = Some((new_window.0, new_window.1));
                    }
                }
                5 => {
                    println!("event 5");
                    if visible_win_id != -1 {
                        let window = map.get(&visible_win_id).unwrap().1.window();
                        window.set_visible(true);
                    }
                }
                6 => {
                    println!("event 6");
                    if visible_win_id != -1 {
                        let window = map.get(&visible_win_id).unwrap().1.window();
                        window.set_visible(false);
                    }
                }
                7 => {
                    println!("event 7");
                    x1 = x;
                    y1 = y;
                    x2 = w;
                    y2 = h;
                }

                _ => {}
            },
            Err(e) => {}
        }
        // *control_flow = ControlFlow::Wait;
        // match event {
        //     Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
        //     Event::WindowEvent {
        //         event: WindowEvent::CloseRequested,
        //         ..
        //     } => {
        //         // *control_flow = ControlFlow::Exit;
        //     }
        //     _ => (),
        // }

        match event {
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => {
                    // webviews.remove(&window_id);
                    // // if webviews.is_empty() {
                    // //     *control_flow = ControlFlow::Exit
                    // // }
                }

                _ => (),
            },
            Event::DeviceEvent {
                device_id, event, ..
            } => match event {
                DeviceEvent::Key(RawKeyEvent {
                    physical_key: KeyCode::Escape,
                    state: ElementState::Pressed,
                }) => match &now_window {
                    Some((id, web)) => {
                        // *control_flow = ControlFlow::Exit;
                        now_window = None;
                    }
                    None => {}
                },
                _ => {}
            },
            Event::UserEvent(UserEvents::NewWindow()) => {
                // let new_window = create_new_window(
                //     format!("Window {}", webviews.len() + 1),
                //     &event_loop,
                //     proxy.clone(),
                //     webviews.len(),
                //     &url,
                // );
                // webviews.insert(new_window.0, new_window.1);
            }
            Event::UserEvent(UserEvents::CloseWindow(id)) => {
                // webviews.remove(&id);
                // if webviews.is_empty() {
                //     *control_flow = ControlFlow::Exit
                // }
            }
            _ => (),
        }
    });
}
