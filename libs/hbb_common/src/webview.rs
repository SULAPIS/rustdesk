use std::collections::HashMap;
use std::ops::Deref;
use wry::{
    application::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        platform::windows::EventLoopExtWindows,
        window::{Window, WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};
#[derive(Clone)]
enum UserEvents {
    CloseWindow(WindowId),
    NewWindow(),
}

pub fn spawn_webview() {
    std::thread::spawn(move || start());
}

fn create_new_window(
    title: String,
    event_loop: EventLoopWindowTarget<UserEvents>,
    proxy: EventLoopProxy<UserEvents>,
) -> (WindowId, WebView) {
    let window = WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();
    let window_id = window.id();
    let event_loop_ = event_loop.clone();
    let handler = move |window: &Window, req: String| match req.as_str() {
        "new-window" => {
            //let _ = proxy.send_event(UserEvents::NewWindow());
            create_new_window("something".into(), event_loop_.clone(), proxy.clone());
        }
        "close" => {
            let _ = proxy.send_event(UserEvents::CloseWindow(window.id()));
        }
        _ if req.starts_with("change-title") => {
            let title = req.replace("change-title:", "");
            window.set_title(title.as_str());
        }
        _ => {}
    };

    let webview = WebViewBuilder::new(window)
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
    (window_id, webview)
}
// #[tokio::main(flavor = "current_thread")]
fn start() {
    let event_loop: EventLoop<UserEvents> = EventLoop::new_any_thread();

    //  let out_event_loop: EventLoopExtWindows<()> = EventLoopExtWindows::new_any_thread();

    let window = WindowBuilder::new()
        .with_title("Hello World")
        .build(&event_loop)
        .unwrap();
    let _webview = WebViewBuilder::new(window)
        .unwrap()
        .with_url("https://tauri.studio")
        .unwrap()
        .build()
        .unwrap();

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}
