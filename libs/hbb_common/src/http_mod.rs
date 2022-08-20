use std::collections::HashMap;

use futures::TryFutureExt;
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

const PRINTER_DEF: i32 = 9999;

pub struct HttpStruct {
    send_info: SendInfo,
    client: Client,
    printers: Vec<Printer>,
}
impl HttpStruct {
    pub fn new() -> Self {
        Self {
            send_info: SendInfo::new(),
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
            printers: Vec::new(),
        }
    }

    async fn start(&mut self) {
        let get_res = self.get_info().await;
        match get_res {
            Ok(res) => {
                if let res_json = res.text_with_charset("json").await {
                    match res_json {
                        Ok(json) => {
                            // println!("{}", json);
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
                            );

                            println!("{:?}", self.send_info);

                            self.post_info().await;
                        }
                        Err(_) => {
                            println!("res_json err");
                        }
                    }
                }
            }
            Err(_) => {
                println!("get_http err");
            }
        }
    }

    async fn get_info(&mut self) -> Result<Response, reqwest::Error> {
        let mut mac_map = HashMap::new();
        mac_map.insert("mac", self.send_info.mac.clone());
        self.client
            .post("http://114.115.156.246:9110/api/terminal/mac")
            .json(&mac_map)
            .header(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            )
            .send()
            .await
    }

    async fn post_info(&self) -> Result<Response, reqwest::Error> {
        self.client
            .post("http://114.115.156.246:9110/api/terminal/save")
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
                println!("{:?}", res);
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
                println!("{:?}", self.printers);
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
    cpuRate: i32,
    memoryVolume: i32,
    memoryAvailable: i32,
    diskVolume: i32,
    diskAvailable: i32,
    ink: String,
    paper: String,
    ip: String,
    mac: String,
    id: String,
    uptDateTime: String,
    pubDate: String,
    uptDate: String,
    pubDateTime: String,
}
#[allow(non_snake_case)]
impl SendInfo {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mac = mac_address::get_mac_address().unwrap().unwrap().to_string();
        sys.refresh_all();

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
    ) {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mac = mac_address::get_mac_address().unwrap().unwrap().to_string();
        sys.refresh_all();

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

        self.cpuRate = cpuRate;
        self.memoryVolume = memoryVolume;
        self.memoryAvailable = memoryAvailable;
        self.diskVolume = diskVolume;
        self.diskAvailable = diskAvailable;
        self.mac = mac;
        self.ip = ip;
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
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        interval.tick().await;
        let _ = http_client.start().await;
    }
}

fn byte_to_g(byte: u64) -> i32 {
    let calc = 1024.0f64.powi(3);
    (byte as f64 / calc) as i32
}

fn kb_to_g(kb: u64) -> i32 {
    (kb as f64 / 1048576.0) as i32
}
