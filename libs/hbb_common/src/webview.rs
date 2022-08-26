use std::collections::HashMap;
use std::ops::Deref;
use wry::{
    application::{
        event::{Event, WindowEvent},
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
    event_loop: &EventLoopWindowTarget<UserEvents>,
    proxy: EventLoopProxy<UserEvents>,
    k: usize,
) -> (WindowId, WebView) {
    let window = WindowBuilder::new()
        .with_decorations(false)
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
    if k >= 2 {
        webview = WebViewBuilder::new(window)
            .unwrap()
            .with_url("http://114.115.156.246:9110")
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
async fn start() {
    let event_loop = EventLoop::<UserEvents>::new_any_thread();

    let mut webviews = HashMap::new();
    let proxy = event_loop.create_proxy();

    let new_window = create_new_window(
        format!("Window {}", webviews.len() + 1),
        &event_loop,
        proxy.clone(),
        webviews.len(),
    );

    webviews.insert(new_window.0, new_window.1);

    //  let out_event_loop: EventLoopExtWindows<()> = EventLoopExtWindows::new_any_thread();

    // let window = WindowBuilder::new()
    //     .with_title("Hello World")
    //     .build(&event_loop)
    //     .unwrap();
    // let _webview = WebViewBuilder::new(window)
    //     .unwrap()
    //     .with_url("http://114.115.156.246:9110/")
    //     .unwrap()
    //     .build()
    //     .unwrap();

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

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
            Event::UserEvent(UserEvents::NewWindow()) => {
                let new_window = create_new_window(
                    format!("Window {}", webviews.len() + 1),
                    &event_loop,
                    proxy.clone(),
                    webviews.len(),
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
