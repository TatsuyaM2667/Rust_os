use alloc::string::String;
use alloc::vec;
use spin::Mutex;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub signal: i8,
    pub security: &'static str,
}

struct WifiState {
    connected_ssid: Option<String>,
}

lazy_static! {
    static ref STATE: Mutex<WifiState> = Mutex::new(WifiState {
        connected_ssid: None,
    });
}

pub fn scan() {
    println!("Scanning for WiFi networks...");
    let networks = vec![
        AccessPoint { ssid: String::from("RustOS-Home"), signal: -45, security: "WPA2" },
        AccessPoint { ssid: String::from("Coffee-Shop-Free"), signal: -68, security: "None" },
        AccessPoint { ssid: String::from("Neighbor-Network"), signal: -82, security: "WPA2" },
    ];

    println!("{:<20} {:<10} {:<10}", "SSID", "SIGNAL", "SECURITY");
    println!("{}", "-".repeat(45));
    for nw in networks {
        println!("{:<20} {:<10} {:<10}", nw.ssid, nw.signal, nw.security);
    }
}

pub fn connect(ssid: &str, _password: &str) {
    println!("Connecting to {}...", ssid);
    for _ in 0..1000000 { unsafe { core::arch::asm!("nop"); } }
    
    let mut state = STATE.lock();
    state.connected_ssid = Some(String::from(ssid));
    println!("Successfully connected to {}.", ssid);
}

pub fn status() {
    let state = STATE.lock();
    if let Some(ref ssid) = state.connected_ssid {
        println!("Connected to: {}", ssid);
        println!("Interface: wlan0 (simulated)");
        println!("IP Address: 192.168.1.42 (simulated)");
    } else {
        println!("WiFi Status: Disconnected");
    }
}
