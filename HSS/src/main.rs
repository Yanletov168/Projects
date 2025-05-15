use clap::Parser;
use colored::*;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use serde_xml_rs::from_str;
use std::io::{self, Write};
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;
use tabular::{Row, Table};
//use base64::encode as base64_encode;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;

#[derive(Debug, Deserialize)]
struct DeviceInfo {
    #[serde(rename = "deviceName")]
    device_name: String,
    #[serde(rename = "serialNumber")]
    serial_number: String,
}

#[derive(Debug)]
struct NetworkDevice {
    ip: String,
    mac: String,
}

#[derive(Debug)]
struct HikvisionDevice {
    ip: String,
    mac: String,
    device_type: String,
    model: String,
    serial: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    username: Option<String>,

    #[arg(short, long)]
    password: Option<String>,

    #[arg(short, long, default_value = "04:03:12,08:54:11,08:A1:89,4C:62:DF,4C:BD:8F,
    08:CC:81,0C:75:D2,10:12:FB,18:68:CB,18:80:25,24:0F:9B,24:28:FD,24:32:AE,24:48:45,
    28:57:BE,2C:A5:9C,34:09:62,3C:1B:F8,40:AC:BF,44:19:B6,44:47:CC,44:A6:42,4C:1F:86,
    4C:F5:DC,50:E5:38,54:8C:81,54:C4:15,58:03:FB,58:50:ED,5C:34:5B,64:DB:8B,68:6D:BC,
    74:3F:C2,80:48:9F,80:7C:62,80:BE:AF,80:F5:AE,84:9A:40,88:DE:39,8C:E7:48,94:E1:AC,
    98:8B:0A,98:9D:E5,98:DF:82,98:F1:12,A0:FF:0C,A4:14:37,A4:29:02,A4:4B:D9,A4:A4:59,
    A4:D5:C2,AC:B9:2F,AC:CB:51,B4:A3:82,BC:5E:33,BC:9B:5E,BC:AD:28,BC:BA:C2,C0:51:7E,
    C0:56:E3,C0:6D:ED,C4:2F:90,C8:A7:02,D4:E8:53,DC:07:F8,DC:D2:6A,E0:BA:AD,E0:CA:3C,
    E0:DF:13,E4:D5:8B,E8:A0:ED,EC:A9:71,EC:C8:9C,F8:4D:FC,FC:9F:FD")]
    oui: String,
}

fn main() {
    let args = Args::parse();
    let (username, password) = get_credentials(args.username, args.password);

    println!("{}", "Начинаем сканирование сети...".green().bold());

    let devices = scan_network(&args.oui);

    if devices.is_empty() {
        println!("{}", "Устройства не найдены. Проверьте сетевые настройки и брандмауэр.".red());
        return;
    }

    let mut hik_devices = Vec::new();
    for device in devices {
        if let Some(info) = get_device_info(&device.ip, &username, &password) {
            let device_type = if info.device_name.contains("NVR") {
                "NVR".to_string()
            } else {
                "Camera".to_string()
            };

            let model = extract_model_from_serial(&info.serial_number);

            hik_devices.push(HikvisionDevice {
                ip: device.ip,
                mac: device.mac,
                device_type,
                model,
                serial: info.serial_number,
            });
        }
    }

    print_devices(&hik_devices);
    interactive_mode(&hik_devices, &username, &password);
}

fn get_credentials(cli_username: Option<String>, cli_password: Option<String>) -> (String, String) {
    let username = cli_username.unwrap_or_else(|| {
        print!("Введите имя пользователя: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    });

    let password = cli_password.unwrap_or_else(|| {
        print!("Введите пароль: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    });

    (username, password)
}

fn scan_network(oui_list: &str) -> Vec<NetworkDevice> {
    let ouis: Vec<&str> = oui_list.split(',').collect();
    let mut devices = Vec::new();

    if is_in_target_subnet() {
        println!("{}", "Хост в целевой подсети, выполняем ARP-сканирование".yellow());
        devices.extend(arp_scan(&ouis));
    }

    if devices.is_empty() {
        println!("{}", "ARP-сканирование не дало результатов, выполняем ICMP-сканирование".yellow());
        devices.extend(icmp_scan(&ouis));
    }

    devices
}

fn is_in_target_subnet() -> bool {
    // Упрощенная проверка - в реальной реализации нужно анализировать сетевые интерфейсы
    if let Ok(local_ip) = local_ip_address::local_ip() {
        if let Ok(target_network) = ipnetwork::IpNetwork::from_str("10.101.0.0/16") {
            return target_network.contains(local_ip);
        }
    }
    false
}

fn arp_scan(ouis: &[&str]) -> Vec<NetworkDevice> {
    let mut devices = Vec::new();
    let output = if cfg!(target_os = "windows") {
        Command::new("arp").arg("-a").output().unwrap().stdout
    } else {
        Command::new("arp").arg("-an").output().unwrap().stdout
    };

    let output = String::from_utf8_lossy(&output);
    let re = Regex::new(r"(?P<ip>\d+\.\d+\.\d+\.\d+).*(?P<mac>([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2}))").unwrap();

    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            let ip = caps["ip"].to_string();
            let mac = caps["mac"].to_string().to_uppercase().replace('-', ":");

            let mac_oui = mac.split(':').take(3).collect::<Vec<_>>().join(":");
            if ouis.contains(&mac_oui.as_str()) {
                println!("Найдено устройство Hikvision: IP={}, MAC={}", ip, mac);
                devices.push(NetworkDevice { ip, mac });
            }
        }
    }

    devices
}

fn icmp_scan(ouis: &[&str]) -> Vec<NetworkDevice> {
    let mut devices = Vec::new();
    println!("Сканируем подсеть 10.101.0.0/16...");

    // В реальной реализации нужно сканировать всю подсеть /16
    // Здесь ограничимся примером для демонстрации
    for i in 2..=50 {
        let ip = format!("10.101.0.{}", i);
        if ping(&ip) {
            println!("Активный IP: {}", ip);
            if let Some(mac) = get_mac_for_ip(&ip) {
                let mac_oui = mac.split(':').take(3).collect::<Vec<_>>().join(":");
                if ouis.contains(&mac_oui.as_str()) {
                    devices.push(NetworkDevice { ip, mac });
                }
            }
        }
    }

    devices
}

fn ping(ip: &str) -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("ping")
            .arg("-n")
            .arg("1")
            .arg("-w")
            .arg("500")
            .arg(ip)
            .output()
            .unwrap()
    } else {
        Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg("1")
            .arg(ip)
            .output()
            .unwrap()
    };

    output.status.success()
}

fn get_mac_for_ip(ip: &str) -> Option<String> {
    // В реальной реализации нужно анализировать ARP-таблицу
    // Здесь просто пример для демонстрации
    if ip == "10.101.0.1" {
        Some("44:19:B6:00:00:01".to_string())
    } else if ip == "10.101.0.2" {
        Some("C0:56:E3:00:00:02".to_string())
    } else {
        None
    }
}

fn get_device_info(ip: &str, username: &str, password: &str) -> Option<DeviceInfo> {
    let url = format!("http://{}/ISAPI/System/deviceInfo", ip);
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let mut headers = HeaderMap::new();
    let auth = format!("{}:{}", username, password);
    let auth_header = format!("Basic {}", BASE64_STANDARD.encode(auth));
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_header).unwrap());

    match client.get(&url).headers(headers).send() {
        Ok(response) => {
            if response.status().is_success() {
                let body = response.text().unwrap();
                match from_str::<DeviceInfo>(&body) {
                    Ok(info) => {
                        println!("Получена информация об устройстве {}: {:?}", ip, info);
                        Some(info)
                    }
                    Err(e) => {
                        println!("Ошибка парсинга ответа от {}: {}", ip, e);
                        None
                    }
                }
            } else {
                println!("Ошибка запроса к {}: {}", ip, response.status());
                None
            }
        }
        Err(e) => {
            println!("Ошибка подключения к {}: {}", ip, e);
            None
        }
    }
}

fn extract_model_from_serial(serial: &str) -> String {
    let parts: Vec<&str> = serial.split('-').collect();
    if parts.len() >= 3 {
        parts[1].to_string()
    } else {
        serial.to_string()
    }
}

fn print_devices(devices: &[HikvisionDevice]) {
    let mut table = Table::new("{:<} {:<} {:<} {:<} {:<}");
    table.add_row(Row::new()
        .with_cell("Тип".bold())
        .with_cell("Модель".bold())
        .with_cell("IP".bold())
        .with_cell("MAC".bold())
        .with_cell("Серийный".bold()));

    // Сначала NVR
    let nvrs: Vec<_> = devices.iter().filter(|d| d.device_type == "NVR").collect();
    for device in nvrs {
        table.add_row(Row::new()
            .with_cell(device.device_type.green())
            .with_cell(device.model.blue())
            .with_cell(device.ip.yellow())
            .with_cell(device.mac.cyan())
            .with_cell(device.serial.clone()));
    }

    // Разделитель с пятью ячейками
    table.add_row(Row::new()
        .with_cell("---")
        .with_cell("---")
        .with_cell("---")
        .with_cell("---")
        .with_cell("---"));

    // Затем камеры
    let cameras: Vec<_> = devices.iter().filter(|d| d.device_type == "Camera").collect();
    for device in cameras {
        table.add_row(Row::new()
            .with_cell(device.device_type.green())
            .with_cell(device.model.blue())
            .with_cell(device.ip.yellow())
            .with_cell(device.mac.cyan())
            .with_cell(device.serial.clone()));
    }

    println!("{}", table);
}

/* 
fn print_devices(devices: &[HikvisionDevice]) {
    let mut table = Table::new("{:<} {:<} {:<} {:<} {:<}");
    table.add_row(Row::new()
        .with_cell("Тип".bold())
        .with_cell("Модель".bold())
        .with_cell("IP".bold())
        .with_cell("MAC".bold())
        .with_cell("Серийный".bold()));

    // Сначала NVR
    let nvrs: Vec<_> = devices.iter().filter(|d| d.device_type == "NVR").collect();
    for device in nvrs {
        table.add_row(Row::new()
            .with_cell(device.device_type.green())
            .with_cell(device.model.blue())
            .with_cell(device.ip.yellow())
            .with_cell(device.mac.cyan())
            .with_cell(device.serial.clone()));
    }

    // Разделитель
    table.add_row(Row::new().with_cell("---".repeat(20)));

    // Затем камеры
    let cameras: Vec<_> = devices.iter().filter(|d| d.device_type == "Camera").collect();
    for device in cameras {
        table.add_row(Row::new()
            .with_cell(device.device_type.green())
            .with_cell(device.model.blue())
            .with_cell(device.ip.yellow())
            .with_cell(device.mac.cyan())
            .with_cell(device.serial.clone()));
    }

    println!("{}", table);
}
*/

fn interactive_mode(devices: &[HikvisionDevice], username: &str, password: &str) {
    println!("\nВведите последние два октета IP (например '17.8') или 'q' для выхода:");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.eq_ignore_ascii_case("q") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        let selected: Vec<_> = input.split(',')
            .map(|s| s.trim())
            .filter_map(|part| {
                let full_ip = format!("10.101.{}", part);
                devices.iter().find(|d| d.ip == full_ip)
            })
            .collect();

        if selected.is_empty() {
            println!("{}", "Устройства не найдены".red());
            continue;
        }

        for device in selected {
            print_device_settings(device, username, password);
        }
    }
}

fn print_device_settings(device: &HikvisionDevice, username: &str, password: &str) {
    println!("\nНастройки устройства {} ({}):", device.ip, device.device_type);

    let endpoints = if device.device_type == "Camera" {
        vec![
            ("Видео", "/ISAPI/Streaming/channels/1"),
            ("Аудио", "/ISAPI/Audio/channels/1"),
            ("Сеть", "/ISAPI/System/Network"),
            ("Изображение", "/ISAPI/Image/channels/1"),
            ("OSD", "/ISAPI/System/Video/inputs/channels/1/overlays/text"),
        ]
    } else {
        vec![
            ("Камеры", "/ISAPI/ContentMgmt/InputProxy/channels"),
        ]
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let auth = format!("{}:{}", username, password);
    let auth_header = format!("Basic {}", BASE64_STANDARD.encode(auth));

    for (name, path) in endpoints {
        let url = format!("http://{}{}", device.ip, path);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_header).unwrap());

        match client.get(&url).headers(headers).send() {
            Ok(response) => {
                if response.status().is_success() {
                    println!("{}: Данные получены", name.green());
                } else {
                    println!("{}: Ошибка запроса ({})", name.yellow(), response.status());
                }
            }
            Err(e) => {
                println!("{}: Ошибка подключения ({})", name.red(), e);
            }
        }
    }
}