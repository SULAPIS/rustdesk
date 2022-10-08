use std::{collections::HashMap, fs::OpenOptions, io::Read};

use mac_address;
use reqwest::{
    header::{HeaderName, HeaderValue},
    Client, Response, StatusCode,
};

use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use sysinfo::{self, CpuExt, DiskExt, System, SystemExt};
use tokio;

///获取config文件信息
///
/// 返回：platform, url
pub fn get_app_url() -> (String, String) {
    let mut contents = String::new();
    {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("config.json")
            .unwrap();
        file.read_to_string(&mut contents).unwrap();
        // println!("{}", contents);
    }
    let config: HashMap<String, String> = serde_json::from_str(&mut contents).unwrap();
    let platform = config.get("platform").unwrap();
    let url = config.get("url").unwrap();
    (platform.clone(), url.clone())
}

const PRINTER_DEF: i32 = 9999;

// pub struct UserInfo {
//     username: String,
//     password: String,
// }

pub struct HttpStruct {
    send_info: SendInfo,
    client: Client,
    printers: Vec<Printer>,
    sysinfo: System,
    url: String,
}
impl HttpStruct {
    pub fn new() -> Self {
        let mut sysinfo = System::new_all();
        Self {
            send_info: SendInfo::new(&mut sysinfo),
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
            printers: Vec::new(),
            sysinfo,
            url: get_app_url().1,
        }
    }

    async fn start(&mut self) {
        let get_res = self
            .get_info(&format!("{}/api/terminal/mac", self.url))
            .await;
        match get_res {
            Ok(res) => {
                if let res_json = res.text_with_charset("json").await {
                    match res_json {
                        Ok(json) => {
                            // println!("rust:http_mod:74: {}", json);
                            let info: Value = serde_json::from_str(&json).unwrap();

                            // println!("{:?}", info);\
                            self.update_printers().await;
                            let (ink, paper) = self.get_min_printer_info();

                            self.send_info.update(
                                ink,
                                paper,
                                info["data"]["uptDateTime"].to_string().replace("\"", ""),
                                info["data"]["pubDate"].to_string().replace("\"", ""),
                                info["data"]["uptDate"].to_string().replace("\"", ""),
                                info["data"]["pubDateTime"].to_string().replace("\"", ""),
                                info["data"]["id"].to_string().replace("\"", ""),
                                &mut self.sysinfo,
                            );

                            // println!("{:?}", self.send_info);

                            self.post_info(&format!("{}/api/terminal/save", self.url))
                                .await;
                        }
                        Err(_) => {
                            println!("res_json err");
                        }
                    }
                }
            }
            Err(_) => {
                println!("rust:http_mod:104: post_http err");
            }
        }
    }

    async fn get_info(&mut self, url: &str) -> Result<Response, reqwest::Error> {
        let mut mac_map = HashMap::new();
        mac_map.insert("mac", self.send_info.mac.clone());
        self.client
            .post(url)
            .json(&mac_map)
            .header(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            )
            .send()
            .await
    }

    async fn post_info(&self, url: &str) -> Result<Response, reqwest::Error> {
        self.client
            .post(url)
            .json(&self.send_info)
            .header(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            )
            .send()
            .await
    }
    async fn update_printers(&mut self) {
        let response = self
            .client
            .get("https://localhost:8081/devices")
            .header(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            )
            .send()
            .await
            .unwrap()
            .text()
            .await;
        match response {
            Ok(res) => {
                println!("rust:http_mod:149: {:?}", res);
                let info: HashMap<String, Value> = serde_json::from_str(&res).unwrap();
                let printers: Vec<Value> =
                    serde_json::from_value(info.get("printers").unwrap().clone()).unwrap();
                let mut printers_vec = Vec::new();

                for printer in printers {
                    let port = printer["portName"].to_string().replace("\"", "");
                    let (ink, paper) = self.get_printer_info(&port).await;
                    printers_vec.push(Printer::new(ink, paper, port));
                }

                self.printers = printers_vec;
                println!("rust:http_mod:149: {:?}", self.printers);
            }
            Err(e) => {
                println!("get printer devices err");
            }
        }
    }

    fn get_min_printer_info(&self) -> (i32, i32) {
        let mut ink = PRINTER_DEF;
        let mut paper = PRINTER_DEF;
        for p in &self.printers {
            ink = ink.min(p.ink);
            paper = paper.min(p.paper);
        }
        (ink, paper)
    }

    async fn get_printer_info(&self, port: &str) -> (i32, i32) {
        let mut ink: i32 = PRINTER_DEF;
        let mut paper: i32 = PRINTER_DEF;
        let response = self
            .client
            .get(format!("https://localhost:8081/status/consumable/{}", port))
            .header(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            )
            .send()
            .await;

        match response {
            Ok(res) => {
                if res.status() == StatusCode::OK {
                    let info = res.text_with_charset("json").await.unwrap();
                    let info: Vec<Value> = serde_json::from_str(&info).unwrap();

                    for v in info {
                        if "ink" == v["ConsumableTypeEnum"].to_string().replace("\"", "") {
                            ink = v["ConsumablePercentageLevelRemaining"]
                                .to_string()
                                .replace("\"", "")
                                .parse()
                                .unwrap_or(0);
                        }

                        if "paper" == v["ConsumableTypeEnum"].to_string().replace("\"", "") {
                            paper = v["ConsumablePercentageLevelRemaining"]
                                .to_string()
                                .replace("\"", "")
                                .parse()
                                .unwrap_or(0);
                        }
                    }
                }
            }
            Err(e) => {
                println!("get printer info err");
            }
        }
        (ink, paper)
    }
}

#[derive(Debug, Clone)]
pub struct Printer {
    port_name: String,
    ink: i32,
    paper: i32,
}

impl Printer {
    pub fn new(ink: i32, paper: i32, port_name: String) -> Self {
        Self {
            port_name,
            ink,
            paper,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct SendInfo {
    pub cpuRate: String,
    pub memoryVolume: String,
    pub memoryAvailable: String,
    pub diskVolume: String,
    pub diskAvailable: String,
    pub ink: String,
    pub paper: String,
    pub ip: String,
    pub mac: String,
    pub id: String,
    pub file: String,
    uptDateTime: String,
    pubDate: String,
    uptDate: String,
    pubDateTime: String,
}
#[allow(non_snake_case)]
impl SendInfo {
    pub fn new(sys: &mut System) -> Self {
        sys.refresh_all();
        let mac = mac_address::get_mac_address().unwrap().unwrap().to_string();

        let mut allDiskSpace = 0;
        let mut availableSpace = 0;
        for disk in sys.disks() {
            allDiskSpace += disk.total_space();
            availableSpace += disk.available_space();
        }
        let diskVolume = byte_to_g(allDiskSpace).to_string();
        let diskAvailable = byte_to_g(availableSpace).to_string();
        let cpuRate = sys.cpus()[0].cpu_usage().to_string();
        let memoryVolume = kb_to_g(sys.total_memory()).to_string();
        let memoryAvailable = kb_to_g(sys.available_memory()).to_string();

        let ip = local_ip().unwrap().to_string();

        SendInfo {
            cpuRate,
            memoryVolume,
            memoryAvailable,
            diskVolume,
            diskAvailable,
            ink: "".to_string(),
            paper: "".to_string(),
            ip,
            mac,
            id: String::new(),
            file: String::new(),
            uptDateTime: String::new(),
            pubDate: String::new(),
            uptDate: String::new(),
            pubDateTime: String::new(),
        }
    }

    pub fn update(
        &mut self,
        ink: i32,
        paper: i32,
        uptDateTime: String,
        pubDate: String,
        uptDate: String,
        pubDateTime: String,
        id: String,
        sys: &mut System,
    ) {
        sys.refresh_all();
        let mac = mac_address::get_mac_address().unwrap().unwrap().to_string();

        let mut allDiskSpace = 0;
        let mut availableSpace = 0;
        for disk in sys.disks() {
            allDiskSpace += disk.total_space();
            availableSpace += disk.available_space();
        }
        let diskVolume = byte_to_g(allDiskSpace);
        let diskAvailable = byte_to_g(availableSpace);
        let cpuRate = sys.cpus()[0].cpu_usage() as i32;
        let memoryVolume = kb_to_g(sys.total_memory());
        let memoryAvailable = kb_to_g(sys.available_memory());
        let ip = local_ip().unwrap().to_string();

        if ink == PRINTER_DEF {
            self.ink = "".to_string();
        } else {
            self.ink = ink.to_string();
        }
        if ink == PRINTER_DEF {
            self.paper = "".to_string();
        } else {
            self.paper = paper.to_string();
        }

        let mut contents = String::new();
        let mut file = OpenOptions::new().read(true).open("path.txt");
        match file {
            Ok(mut f) => {
                f.read_to_string(&mut contents).unwrap();
            }
            Err(_) => todo!(),
        }

        // println!("rust:http_mod.rs:348: <{}>", contents);

        self.cpuRate = cpuRate.to_string();
        self.memoryVolume = memoryVolume.to_string();
        self.memoryAvailable = memoryAvailable.to_string();
        self.diskVolume = diskVolume.to_string();
        self.diskAvailable = diskAvailable.to_string();
        self.mac = mac;
        self.ip = ip;
        self.file = contents;
        self.uptDate = uptDate;
        self.pubDate = pubDate;
        self.pubDateTime = pubDateTime;
        self.uptDateTime = uptDateTime;
        self.id = id;
    }
}

pub fn spawn_http() {
    std::thread::spawn(move || start());
}

#[tokio::main(flavor = "current_thread")]
async fn start() {
    let mut http_client = HttpStruct::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        let _ = http_client.start().await;
        // http_client.

        // let _=token_post_info("http://114.115.156.246:9110/api/platform/caches").await;
    }
}

//  async fn token_post_info(url:&str) -> Result<Response, reqwest::Error> {
//         self.client
//             .post(str)
//             .json(&self.send_info)
//             .header(
//                 HeaderName::from_static("content-type"),
//                 HeaderValue::from_static("application/json"),
//             )
//             .send()
//             .await
//     }

fn byte_to_g(byte: u64) -> i32 {
    let calc = 1024.0f64.powi(3);
    (byte as f64 / calc) as i32
}

fn kb_to_g(kb: u64) -> i32 {
    (kb as f64 / 1048576.0) as i32
}
