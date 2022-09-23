use std::{
    collections::HashMap,
    fs::OpenOptions,
    sync::{Arc, Mutex},
};
use trayicon::{MenuBuilder, TrayIconBuilder};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

#[derive(Clone, Eq, PartialEq, Debug)]
enum Events {
    DoubleClickTrayIcon,
    StopService,
    StartService,
    MaxSize,
}

pub fn start_tray(options: Arc<Mutex<HashMap<String, String>>>) {
    println!("Starting tray...");
    let event_loop = EventLoop::<Events>::with_user_event();
    let proxy = event_loop.create_proxy();
    let icon = include_bytes!("./tray-icon.ico");
    let mut tray_icon = TrayIconBuilder::new()
        .sender_winit(proxy)
        .icon_from_buffer(icon)
        .tooltip("青岛政务")
        .on_double_click(Events::DoubleClickTrayIcon)
        .on_click(Events::MaxSize)
        .build()
        .unwrap();
    let old_state = Arc::new(Mutex::new(0));
    let mut count = 0;

    event_loop.run(move |event, _, control_flow| {
        if options.lock().unwrap().get("ipc-closed").is_some() {
            *control_flow = ControlFlow::Exit;
            return;
        } else {
            *control_flow = ControlFlow::Wait;
        }
        let stopped = if let Some(v) = options.lock().unwrap().get("stop-service") {
            !v.is_empty()
        } else {
            false
        };
        let stopped = if stopped { 2 } else { 1 };
        let old = *old_state.lock().unwrap();
        if stopped != old {
            hbb_common::log::info!("State changed");
            let mut m = MenuBuilder::new();
            // if stopped == 2 {
            //     m = m.item(
            //         &crate::client::translate("Start service".to_owned()),
            //         Events::StartService,
            //     );
            // } else {
            // m = m.item(
            //     &crate::client::translate("Stop service".to_owned()),
            //     Events::StopService,
            // );
            m = m.item("退出", Events::StopService);
            // }
            tray_icon.set_menu(&m).ok();
            *old_state.lock().unwrap() = stopped;
        }
        if count == 0 {
            crate::run_me(Vec::<&str>::new()).ok();
            count = 1;
        }
        if count == 100 {
            count = 1;
            use std::fs::File;
            use std::io::prelude::*;
            {
                let mut contents = String::new();
                let mut file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open("state.txt")
                    .unwrap();
                file.read_to_string(&mut contents).unwrap();
                if contents == "app_quit" {
                    let mut file = File::create("state.txt").unwrap();
                    std::process::exit(0);
                }
            }

            println!("click");
        }
        count += 1;
        match event {
            Event::UserEvent(e) => match e {
                Events::MaxSize => {
                    // crate::ipc::win_maxsize();
                    use std::fs::File;
                    use std::io::prelude::*;
                    {
                        let mut contents = String::new();
                        let mut file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open("state.txt")
                            .unwrap();
                        file.read_to_string(&mut contents).unwrap();
                        if contents == "" {
                            let mut file = File::create("state.txt").unwrap();
                            file.write_all(b"minsize").unwrap();
                        }
                    }

                    println!("click");
                }
                Events::DoubleClickTrayIcon => {
                    use std::fs::File;
                    use std::io::prelude::*;
                    {
                        let mut file = File::create("state.txt").unwrap();
                        file.write_all(b"maxsize").unwrap();
                    }
                }
                Events::StopService => {
                    // crate::ipc::set_option("stop-service", "Y");
                    use std::fs::File;
                    use std::io::prelude::*;
                    {
                        let mut file = File::create("state.txt").unwrap();
                        file.write_all(b"exit").unwrap();
                    }
                    std::process::exit(0);
                }
                Events::StartService => {
                    crate::ipc::set_option("stop-service", "");
                }
            },
            _ => (),
        }
    });
}
