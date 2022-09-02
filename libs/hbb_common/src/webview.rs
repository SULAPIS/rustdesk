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

pub fn spawn_webview(url: String, rx: Receiver<(i32, i32)>) {
    std::thread::spawn(move || start(url, rx));
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
        .with_always_on_top(true)
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
    if k >= 0 {
        webview = WebViewBuilder::new(window)
            .unwrap()
            .with_url(url)
            .unwrap()
            .with_ipc_handler(handler)
            .build()
            .unwrap();
    } else {
        webview = WebViewBuilder::new(window)
            .unwrap()
            .with_html(
                r#"
          <button onclick="window.ipc.postMessage('new-window')">Open a new window</button>
          <button onclick="window.ipc.postMessage('close')">Close current window</button>
          <input oninput="window.ipc.postMessage(`change-title:${this.value}`)" />
      "#,
            )
            .unwrap()
            .with_ipc_handler(handler)
            .build()
            .unwrap();
    }
    (window_id, webview)
}

// struct Webviews{
//     webview:HashMap<WindowId,WebView>,
//     num
// }

#[tokio::main(flavor = "current_thread")]
async fn start(url: String, rx: Receiver<(i32, i32)>) {
    let event_loop = EventLoop::<UserEvents>::new_any_thread();

    let mut webviews = HashMap::new();
    let proxy = event_loop.create_proxy();

    let new_window = create_new_window(
        format!("Window {}", webviews.len() + 1),
        &event_loop,
        proxy.clone(),
        webviews.len(),
        &url,
    );

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;
        let window = new_window.1.window();
        let mut pos = window.outer_position().unwrap();
        let rec = rx.try_recv();
        match rec {
            Ok((x, y)) => {
                println!("x: {},y: {}", x, y);
                pos.x = x + 80;
                pos.y = y + 58;
                window.set_outer_position(pos);
            }
            Err(e) => {}
        }

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
                    webviews.remove(&window_id);
                    // if webviews.is_empty() {
                    //     *control_flow = ControlFlow::Exit
                    // }
                }

                _ => (),
            },
            Event::DeviceEvent {
                device_id, event, ..
            } => match event {
                DeviceEvent::Key(RawKeyEvent {
                    physical_key: KeyCode::Escape,
                    state: ElementState::Pressed,
                }) => {
                    webviews.clear();
                }
                _ => {}
            },
            Event::UserEvent(UserEvents::NewWindow()) => {
                let new_window = create_new_window(
                    format!("Window {}", webviews.len() + 1),
                    &event_loop,
                    proxy.clone(),
                    webviews.len(),
                    &url,
                );
                webviews.insert(new_window.0, new_window.1);
            }
            Event::UserEvent(UserEvents::CloseWindow(id)) => {
                webviews.remove(&id);
                // if webviews.is_empty() {
                //     *control_flow = ControlFlow::Exit
                // }
            }
            _ => (),
        }
    });
}
