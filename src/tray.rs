use hbb_common::http_mod;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    rc::Rc,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
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

const FILES_NAME: [&str; 7] = [
    "login_logo",
    "login_screen",
    "home_logo",
    "background",
    "title",
    "home_left",
    "table",
];

const FILES_FIELD: [&str; 7] = [
    "loginLogoAttachmentUrl",
    "loginScreenAttachmentUrl",
    "homeLogoAttachmentUrl",
    "naviBackgroundAttachmentUrl",
    "naviTitleAttachmentUrl",
    "homeLeftAttachmentUrl",
    "tableHeadAttachmentUrl",
];
fn load_config() -> Vec<JoinHandle<()>> {
    let (platform, url) = hbb_common::http_mod::get_app_url();
    let config_res = http_request(&url, "/api/platform/caches");
    let mut handles = vec![];

    match config_res {
        Ok(res) => {
            let info = res.text_with_charset("json").unwrap();
            // println!("{}", info);

            let info: HashMap<String, serde_json::Value> = serde_json::from_str(&info).unwrap();
            let data = info.get("data").unwrap();
            let data: Vec<serde_json::Value> = serde_json::from_value(data.clone()).unwrap();

            for d in data {
                let v = d["code"].to_string().replace("\"", "");
                if v == platform {
                    for (i, &file_field) in FILES_FIELD.iter().enumerate() {
                        let file_name = FILES_NAME[i].to_string();
                        let url = d[file_field].to_string().replace("\"", "");
                        let handle = thread::spawn(move || {
                            crate_file(&url, &file_name);
                        });
                        handles.push(handle);
                    }
                }
            }
        }
        Err(_) => {
            println!("下载config图片失败")
        }
    }

    println!("rust:tray.rs:75: <{} {}>", platform, url);
    handles
}
fn http_request(url: &str, params: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    client.post(format!("{}{}", url, params)).send()
}

fn crate_file(url: &str, name: &str) {
    let mut out = std::fs::File::create(format!("src/ui/pic/{}.png", name))
        .expect(" tray line 27: failed to create file");
    let resp = reqwest::blocking::get(url)
        .expect("request failed")
        .copy_to(&mut out);
    println!("rust:main.rs:59: <{}.png>", name);
}
pub fn start_tray(options: Arc<Mutex<HashMap<String, String>>>) {
    println!("Starting tray...");
    println!("Download Config image...");
    let handles = load_config();
    for handle in handles {
        handle.join().unwrap();
    }
    http_mod::spawn_http();

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
            // Events::StartService,
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

            // println!("click");
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

                    // println!("click");
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
