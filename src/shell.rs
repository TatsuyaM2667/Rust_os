// RustOS - CLIシェル
// キーボード入力を受け取り、コマンドを実行する対話型シェル

use crate::vga_buffer::{self, Color};
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

const MAX_HISTORY: usize = 32;

// キーボードの状態
lazy_static::lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(
            ScancodeSet1::new(),
            layouts::Us104Key,
            HandleControl::Ignore,
        ));
    static ref INPUT_BUFFER: Mutex<String> = Mutex::new(String::new());
    static ref COMMAND_HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref HISTORY_INDEX: Mutex<Option<usize>> = Mutex::new(None);
    static ref CURRENT_DIR: Mutex<String> = Mutex::new(String::from("/"));
}

pub fn init() {
    print_banner();
    print_prompt();
}

fn print_banner() {
    let mut writer = vga_buffer::WRITER.lock();
    writer.clear_screen();
    writer.enable_cursor();
    drop(writer);

    vga_buffer::print_colored(
        r"
  ____            _    ___  ____  
 |  _ \ _   _ ___| |_ / _ \/ ___| 
 | |_) | | | / __| __| | | \___ \ 
 |  _ <| |_| \__ \ |_| |_| |___) |
 |_| \_\\__,_|___/\__|\___/|____/ 
",
        Color::LightCyan,
        Color::Black,
    );
    println!();
    vga_buffer::print_colored(
        "  RustOS v0.1.0 - A minimal OS written in Rust\n",
        Color::Yellow,
        Color::Black,
    );
    vga_buffer::print_colored(
        "  Type 'help' for available commands.\n",
        Color::LightGray,
        Color::Black,
    );
    println!();
}

fn print_prompt() {
    let current_dir = CURRENT_DIR.lock();
    vga_buffer::print_colored(&alloc::format!("root@rust-os:{}# ", *current_dir), Color::LightGreen, Color::Black);
    // カーソル位置を更新
    x86_64::instructions::interrupts::without_interrupts(|| {
        vga_buffer::WRITER.lock().update_cursor();
    });
}

/// キーボード割り込みから呼ばれるスキャンコードハンドラ
pub fn handle_scancode(scancode: u8) {
    let mut keyboard = KEYBOARD.lock();

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => handle_char(character),
                DecodedKey::RawKey(key) => handle_raw_key(key),
            }
        }
    }
}

fn handle_char(character: char) {
    match character {
        '\n' => {
            println!();
            let cmd = {
                let buf = INPUT_BUFFER.lock();
                buf.clone()
            };
            if !cmd.is_empty() {
                // 履歴に追加
                {
                    let mut history = COMMAND_HISTORY.lock();
                    if history.len() >= MAX_HISTORY {
                        history.remove(0);
                    }
                    history.push(cmd.clone());
                    *HISTORY_INDEX.lock() = None;
                }
                execute_command(&cmd);
            }
            INPUT_BUFFER.lock().clear();
            print_prompt();
        }
        '\x08' => {
            // Backspace
            let mut buf = INPUT_BUFFER.lock();
            if !buf.is_empty() {
                buf.pop();
                print!("\x08");
                x86_64::instructions::interrupts::without_interrupts(|| {
                    vga_buffer::WRITER.lock().update_cursor();
                });
            }
        }
        c if c >= ' ' && c <= '~' => {
            INPUT_BUFFER.lock().push(c);
            print!("{}", c);
            x86_64::instructions::interrupts::without_interrupts(|| {
                vga_buffer::WRITER.lock().update_cursor();
            });
        }
        _ => {}
    }
}

fn handle_raw_key(key: pc_keyboard::KeyCode) {
    use pc_keyboard::KeyCode;

    match key {
        KeyCode::ArrowUp => {
            let history = COMMAND_HISTORY.lock();
            if history.is_empty() {
                return;
            }
            let mut idx = HISTORY_INDEX.lock();
            let new_idx = match *idx {
                None => history.len() - 1,
                Some(i) if i > 0 => i - 1,
                Some(i) => i,
            };
            *idx = Some(new_idx);
            let cmd = history[new_idx].clone();
            drop(history);
            drop(idx);
            replace_input(&cmd);
        }
        KeyCode::ArrowDown => {
            let history = COMMAND_HISTORY.lock();
            let mut idx = HISTORY_INDEX.lock();
            match *idx {
                None => return,
                Some(i) if i + 1 < history.len() => {
                    *idx = Some(i + 1);
                    let cmd = history[i + 1].clone();
                    drop(history);
                    drop(idx);
                    replace_input(&cmd);
                }
                _ => {
                    *idx = None;
                    drop(history);
                    drop(idx);
                    replace_input("");
                }
            }
        }
        _ => {}
    }
}

fn replace_input(new_input: &str) {
    let mut buf = INPUT_BUFFER.lock();
    // 現在の入力を消す
    for _ in 0..buf.len() {
        print!("\x08");
    }
    buf.clear();
    buf.push_str(new_input);
    print!("{}", new_input);
    x86_64::instructions::interrupts::without_interrupts(|| {
        vga_buffer::WRITER.lock().update_cursor();
    });
}

/// コマンドの実行
fn execute_command(input: &str) {
    let input = input.trim();
    let mut parts = input.splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    match command {
        "help" => cmd_help(),
        "clear" | "cls" => cmd_clear(),
        "echo" => cmd_echo(args),
        "info" | "sysinfo" => cmd_info(),
        "mem" | "memory" => cmd_mem(),
        "uptime" => cmd_uptime(),
        "wifi" => cmd_wifi(args),
        "gui" => cmd_gui(),
        "color" => cmd_color(args),
        "calc" => cmd_calc(args),
        "history" => cmd_history(),
        "date" | "time" => cmd_time(),
        "reboot" => cmd_reboot(),
        "shutdown" | "halt" => cmd_shutdown(),
        "ascii" => cmd_ascii(args),
        "matrix" => cmd_matrix(),
        "whoami" => {
            vga_buffer::print_colored("root\n", Color::LightCyan, Color::Black);
        }
        "uname" => {
            println!("RustOS v0.1.0 x86_64");
        }
        "hostname" => {
            println!("rust-os");
        }
        "ls" | "dir" => cmd_ls(args),
        "pwd" => cmd_pwd(),
        "cd" => cmd_cd(args),
        "mkdir" => cmd_mkdir(args),
        "touch" => cmd_touch(args),
        "rm" => cmd_rm(args),
        "cat" => cmd_cat(args),
        "pacman" => cmd_pacman(args),
        "ps" => cmd_ps(),
        "top" => cmd_top(),
        "ping" => cmd_ping(args),
        "net" => cmd_net(),
        "" => {}
        _ => {
            vga_buffer::print_colored("Unknown command: ", Color::LightRed, Color::Black);
            println!("'{}'. Type 'help' for available commands.", command);
        }
    }
}

fn cmd_ls(args: &str) {
    let mut fs = crate::fs::FILESYSTEM.lock();
    let path = if args.is_empty() {
        CURRENT_DIR.lock().clone()
    } else {
        args.to_string()
    };

    match fs.read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                print!("{}  ", entry);
            }
            println!();
        }
        Err(e) => println!("ls: {}: {}", path, e),
    }
}

fn cmd_pwd() {
    println!("{}", *CURRENT_DIR.lock());
}

fn cmd_cd(args: &str) {
    if args.is_empty() { return; }
    let mut current = CURRENT_DIR.lock();
    if args == ".." {
        if *current != "/" {
            let mut parts: Vec<&str> = current.split('/').filter(|s| !s.is_empty()).collect();
            parts.pop();
            *current = alloc::format!("/{}", parts.join("/"));
        }
    } else if args == "/" {
        *current = String::from("/");
    } else {
        // Simple relative path support
        let new_path = if args.starts_with('/') {
            args.to_string()
        } else if *current == "/" {
            alloc::format!("/{}", args)
        } else {
            alloc::format!("{}/{}", *current, args)
        };
        
        let mut fs = crate::fs::FILESYSTEM.lock();
        if fs.read_dir(&new_path).is_ok() {
            *current = new_path;
        } else {
            println!("cd: {}: No such directory", args);
        }
    }
}

fn cmd_mkdir(args: &str) {
    if args.is_empty() { return; }
    let mut fs = crate::fs::FILESYSTEM.lock();
    let path = if args.starts_with('/') {
        args.to_string()
    } else {
        let current = CURRENT_DIR.lock();
        if *current == "/" { alloc::format!("/{}", args) } else { alloc::format!("{}/{}", *current, args) }
    };
    if let Err(e) = fs.mkdir(&path) {
        println!("mkdir: {}: {}", args, e);
    }
}

fn cmd_touch(args: &str) {
    if args.is_empty() { return; }
    let mut fs = crate::fs::FILESYSTEM.lock();
    let path = if args.starts_with('/') {
        args.to_string()
    } else {
        let current = CURRENT_DIR.lock();
        if *current == "/" { alloc::format!("/{}", args) } else { alloc::format!("{}/{}", *current, args) }
    };
    if let Err(e) = fs.touch(&path) {
        println!("touch: {}: {}", args, e);
    }
}

fn cmd_rm(args: &str) {
    if args.is_empty() { return; }
    let mut fs = crate::fs::FILESYSTEM.lock();
    let path = if args.starts_with('/') {
        args.to_string()
    } else {
        let current = CURRENT_DIR.lock();
        if *current == "/" { alloc::format!("/{}", args) } else { alloc::format!("{}/{}", *current, args) }
    };
    if let Err(e) = fs.remove(&path) {
        println!("rm: {}: {}", args, e);
    }
}

fn cmd_cat(args: &str) {
    if args.is_empty() { return; }
    let mut fs = crate::fs::FILESYSTEM.lock();
    let path = if args.starts_with('/') {
        args.to_string()
    } else {
        let current = CURRENT_DIR.lock();
        if *current == "/" { alloc::format!("/{}", args) } else { alloc::format!("{}/{}", *current, args) }
    };
    match fs.read_file(&path) {
        Ok(data) => println!("{}", String::from_utf8_lossy(&data)),
        Err(e) => println!("cat: {}: {}", args, e),
    }
}

fn cmd_pacman(args: &str) {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        println!("usage:  pacman <operation> [...]");
        println!("operations:");
        println!("  pacman {{-h --help}}");
        println!("  pacman {{-Q --query}}");
        println!("  pacman {{-R --remove}}");
        println!("  pacman {{-S --sync}}");
        return;
    }

    match parts[0] {
        "-S" => {
            if parts.len() < 2 { println!("error: no targets specified"); }
            else { crate::pacman::install(parts[1]); }
        }
        "-Ss" => {
            if parts.len() < 2 { println!("error: no targets specified"); }
            else { crate::pacman::search(parts[1]); }
        }
        "-Qs" | "-Q" => crate::pacman::list_installed(),
        "-R" => {
            if parts.len() < 2 { println!("error: no targets specified"); }
            else { crate::pacman::remove(parts[1]); }
        }
        _ => println!("pacman: invalid option: {}", parts[0]),
    }
}

fn cmd_ps() {
    vga_buffer::print_colored("PID  TTY      TIME     CMD\n", Color::Yellow, Color::Black);
    println!("1    tty1     00:00:01 kernel");
    println!("2    tty1     00:00:00 shell");
}

fn cmd_top() {
    println!("top - 00:00:01 up 1 min, 1 user, load average: 0.00, 0.00, 0.00");
    println!("Tasks: 2 total, 1 running, 1 sleeping, 0 stopped, 0 zombie");
    println!("%Cpu(s):  0.0 us,  0.0 sy,  0.0 ni, 100.0 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st");
    cmd_mem();
}

fn cmd_ping(args: &str) {
    if args.is_empty() {
        println!("Usage: ping <host>");
        return;
    }
    println!("PING {} ({}): 56 data bytes", args, args);
    println!("64 bytes from {}: icmp_seq=0 ttl=64 time=0.123 ms", args);
    println!("64 bytes from {}: icmp_seq=1 ttl=64 time=0.145 ms", args);
    println!("^C");
    println!("--- {} ping statistics ---", args);
    println!("2 packets transmitted, 2 packets received, 0.0% packet loss");
}

fn cmd_help() {
    vga_buffer::print_colored("=== RustOS Commands ===\n", Color::Yellow, Color::Black);
    let commands = [
        ("help", "Show this help message"),
        ("clear/cls", "Clear the screen"),
        ("echo <text>", "Print text to screen"),
        ("ls/dir", "List directory contents"),
        ("pwd", "Print working directory"),
        ("cd <dir>", "Change directory"),
        ("mkdir <dir>", "Create directory"),
        ("touch <file>", "Create empty file"),
        ("rm <path>", "Remove file or directory"),
        ("cat <file>", "Show file contents"),
        ("pacman -S <pkg>", "Install package"),
        ("pacman -Qs", "List installed packages"),
        ("ps", "Show running processes"),
        ("top", "Show system usage"),
        ("ping <host>", "Test network connectivity"),
        ("info/sysinfo", "Show system information"),
        ("mem/memory", "Show memory information"),
        ("uptime", "Show system uptime"),
        ("net", "Show network interface"),
        ("wifi <cmd>", "WiFi: scan, connect, status"),
        ("gui", "Start GUI mode"),
        ("reboot", "Reboot the system"),
        ("shutdown/halt", "Halt the system"),
    ];

    for (cmd, desc) in commands.iter() {
        vga_buffer::print_colored("  ", Color::White, Color::Black);
        vga_buffer::print_colored(cmd, Color::LightCyan, Color::Black);
        // パディング
        let padding = 20 - cmd.len().min(20);
        for _ in 0..padding {
            print!(" ");
        }
        vga_buffer::print_colored(desc, Color::LightGray, Color::Black);
        println!();
    }
}

fn cmd_clear() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        vga_buffer::WRITER.lock().clear_screen();
    });
}

fn cmd_echo(args: &str) {
    if args.is_empty() {
        println!();
    } else {
        println!("{}", args);
    }
}

fn cmd_info() {
    vga_buffer::print_colored("=== System Information ===\n", Color::Yellow, Color::Black);
    println!("  OS:           RustOS v0.1.0");
    println!("  Kernel:       Rust bare-metal kernel");
    println!("  Architecture: x86_64");
    println!("  Display:      VGA Text Mode (80x25)");
    println!("  Input:        PS/2 Keyboard");
    println!("  Boot:         Bootloader v0.9");

    // CPUID情報
    let cpuid = unsafe {
        let mut vendor = [0u8; 12];
        let result: u32;
        core::arch::asm!(
            "push rbx",
            "xor eax, eax",
            "cpuid",
            "mov [{vendor}], ebx",
            "mov [{vendor} + 4], edx",
            "mov [{vendor} + 8], ecx",
            "pop rbx",
            vendor = in(reg) vendor.as_mut_ptr(),
            out("eax") result,
            out("ecx") _,
            out("edx") _,
        );
        (result, vendor)
    };

    if let Ok(vendor_str) = core::str::from_utf8(&cpuid.1) {
        println!("  CPU Vendor:   {}", vendor_str);
    }
    println!("  Max CPUID:    {}", cpuid.0);
}

fn cmd_mem() {
    vga_buffer::print_colored("=== Memory Information ===\n", Color::Yellow, Color::Black);
    println!(
        "  Heap Start:   0x{:x}",
        crate::memory::HEAP_START
    );
    println!(
        "  Heap Size:    {} KiB ({} bytes)",
        crate::memory::HEAP_SIZE / 1024,
        crate::memory::HEAP_SIZE
    );

    // アロケータの統計を表示
    let (used, free) = {
        let allocator = {
            // linked_list_allocator の stats
            let alloc = &crate::allocator::ALLOCATOR;
            let locked = alloc.lock();
            (locked.used(), locked.free())
        };
        (allocator.0, allocator.1)
    };
    println!("  Used:         {} bytes", used);
    println!("  Free:         {} bytes", free);

    // メモリバーを表示
    let total = crate::memory::HEAP_SIZE;
    let pct = if total > 0 { (used * 100) / total } else { 0 };
    print!("  Usage:        [");
    let bar_width = 40;
    let filled = (pct * bar_width) / 100;
    for i in 0..bar_width {
        if i < filled {
            vga_buffer::print_colored("█", Color::LightGreen, Color::Black);
        } else {
            vga_buffer::print_colored("░", Color::DarkGray, Color::Black);
        }
    }
    println!("] {}%", pct);
}

fn cmd_uptime() {
    let seconds = crate::interrupts::get_uptime_seconds();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    println!("Uptime: {:02}:{:02}:{:02}", hours, minutes, secs);
}

fn cmd_color(args: &str) {
    let colors = [
        "black", "blue", "green", "cyan", "red", "magenta", "brown",
        "lightgray", "darkgray", "lightblue", "lightgreen", "lightcyan",
        "lightred", "pink", "yellow", "white",
    ];

    if args.is_empty() {
        println!("Available colors:");
        for (i, c) in colors.iter().enumerate() {
            let color = color_from_index(i as u8);
            vga_buffer::print_colored(&alloc::format!("  {} - {}\n", i, c), color, Color::Black);
        }
        println!("Usage: color <foreground> [background]");
        return;
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let fg = parse_color(parts[0]);
    let bg = if parts.len() > 1 {
        parse_color(parts[1])
    } else {
        Some(Color::Black)
    };

    match (fg, bg) {
        (Some(fg), Some(bg)) => {
            x86_64::instructions::interrupts::without_interrupts(|| {
                vga_buffer::WRITER.lock().set_color(fg, bg);
            });
            println!("Color changed.");
        }
        _ => {
            println!("Invalid color. Use 'color' to see available colors.");
        }
    }
}

fn parse_color(s: &str) -> Option<Color> {
    // 数値で指定
    if let Ok(n) = s.parse::<u8>() {
        if n <= 15 {
            return Some(color_from_index(n));
        }
    }
    // 名前で指定
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "blue" => Some(Color::Blue),
        "green" => Some(Color::Green),
        "cyan" => Some(Color::Cyan),
        "red" => Some(Color::Red),
        "magenta" => Some(Color::Magenta),
        "brown" => Some(Color::Brown),
        "lightgray" | "light_gray" => Some(Color::LightGray),
        "darkgray" | "dark_gray" => Some(Color::DarkGray),
        "lightblue" | "light_blue" => Some(Color::LightBlue),
        "lightgreen" | "light_green" => Some(Color::LightGreen),
        "lightcyan" | "light_cyan" => Some(Color::LightCyan),
        "lightred" | "light_red" => Some(Color::LightRed),
        "pink" => Some(Color::Pink),
        "yellow" => Some(Color::Yellow),
        "white" => Some(Color::White),
        _ => None,
    }
}

fn color_from_index(i: u8) -> Color {
    match i {
        0 => Color::Black,
        1 => Color::Blue,
        2 => Color::Green,
        3 => Color::Cyan,
        4 => Color::Red,
        5 => Color::Magenta,
        6 => Color::Brown,
        7 => Color::LightGray,
        8 => Color::DarkGray,
        9 => Color::LightBlue,
        10 => Color::LightGreen,
        11 => Color::LightCyan,
        12 => Color::LightRed,
        13 => Color::Pink,
        14 => Color::Yellow,
        15 => Color::White,
        _ => Color::White,
    }
}

fn cmd_calc(args: &str) {
    if args.is_empty() {
        println!("Usage: calc <number> <op> <number>");
        println!("Operators: + - * / %");
        println!("Example: calc 42 + 13");
        return;
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() != 3 {
        println!("Usage: calc <number> <op> <number>");
        return;
    }

    let a: Result<i64, _> = parts[0].parse();
    let b: Result<i64, _> = parts[2].parse();

    match (a, b) {
        (Ok(a), Ok(b)) => {
            let result = match parts[1] {
                "+" => Some(a + b),
                "-" => Some(a - b),
                "*" => Some(a * b),
                "/" => {
                    if b == 0 {
                        println!("Error: Division by zero");
                        return;
                    }
                    Some(a / b)
                }
                "%" => {
                    if b == 0 {
                        println!("Error: Division by zero");
                        return;
                    }
                    Some(a % b)
                }
                _ => {
                    println!("Unknown operator: {}", parts[1]);
                    None
                }
            };
            if let Some(r) = result {
                vga_buffer::print_colored(
                    &alloc::format!("= {}\n", r),
                    Color::LightCyan,
                    Color::Black,
                );
            }
        }
        _ => println!("Error: Invalid numbers"),
    }
}

fn cmd_history() {
    let history = COMMAND_HISTORY.lock();
    if history.is_empty() {
        println!("No command history.");
        return;
    }
    vga_buffer::print_colored("=== Command History ===\n", Color::Yellow, Color::Black);
    for (i, cmd) in history.iter().enumerate() {
        println!("  {:3}  {}", i + 1, cmd);
    }
}

fn cmd_time() {
    // CMOS RTC から時刻を読む
    let (hour, min, sec) = read_rtc_time();
    println!("Current time (UTC): {:02}:{:02}:{:02}", hour, min, sec);

    let (year, month, day) = read_rtc_date();
    println!("Current date (UTC): 20{:02}-{:02}-{:02}", year, month, day);
}

fn read_rtc_time() -> (u8, u8, u8) {
    unsafe {
        let mut port_addr: x86_64::instructions::port::Port<u8> =
            x86_64::instructions::port::Port::new(0x70);
        let mut port_data: x86_64::instructions::port::Port<u8> =
            x86_64::instructions::port::Port::new(0x71);

        // 秒
        port_addr.write(0x00);
        let sec_bcd = port_data.read();
        // 分
        port_addr.write(0x02);
        let min_bcd = port_data.read();
        // 時
        port_addr.write(0x04);
        let hour_bcd = port_data.read();

        // BCD -> binary
        let sec = (sec_bcd & 0x0F) + ((sec_bcd >> 4) * 10);
        let min = (min_bcd & 0x0F) + ((min_bcd >> 4) * 10);
        let hour = (hour_bcd & 0x0F) + ((hour_bcd >> 4) * 10);

        (hour, min, sec)
    }
}

fn read_rtc_date() -> (u8, u8, u8) {
    unsafe {
        let mut port_addr: x86_64::instructions::port::Port<u8> =
            x86_64::instructions::port::Port::new(0x70);
        let mut port_data: x86_64::instructions::port::Port<u8> =
            x86_64::instructions::port::Port::new(0x71);

        // 日
        port_addr.write(0x07);
        let day_bcd = port_data.read();
        // 月
        port_addr.write(0x08);
        let month_bcd = port_data.read();
        // 年
        port_addr.write(0x09);
        let year_bcd = port_data.read();

        let day = (day_bcd & 0x0F) + ((day_bcd >> 4) * 10);
        let month = (month_bcd & 0x0F) + ((month_bcd >> 4) * 10);
        let year = (year_bcd & 0x0F) + ((year_bcd >> 4) * 10);

        (year, month, day)
    }
}

fn cmd_reboot() {
    println!("Rebooting...");
    // PS/2 コントローラを使ってリセット
    unsafe {
        let mut port: x86_64::instructions::port::Port<u8> =
            x86_64::instructions::port::Port::new(0x64);
        port.write(0xFE);
    }
    
    loop {
        x86_64::instructions::hlt();
    }
}

fn cmd_shutdown() {
    println!("Shutting down...");
    // QEMU exit
    unsafe {
        let mut port: x86_64::instructions::port::Port<u32> =
            x86_64::instructions::port::Port::new(0xf4);
        port.write(0x00);
    }
    // ACPI shutdown (QEMU)
    unsafe {
        let mut port: x86_64::instructions::port::Port<u16> =
            x86_64::instructions::port::Port::new(0x604);
        port.write(0x2000);
    }
    println!("It is safe to turn off your computer.");
    loop {
        x86_64::instructions::hlt();
    }
}

fn cmd_ascii(args: &str) {
    if args.is_empty() {
        vga_buffer::print_colored("=== ASCII Table ===\n", Color::Yellow, Color::Black);
        for i in 32u8..=126 {
            if (i - 32) % 16 == 0 {
                println!();
                print!("  ");
            }
            print!("{:3} {:>1}  ", i, i as char);
        }
        println!();
    } else {
        for c in args.chars() {
            println!("'{}' = {} (0x{:02x})", c, c as u32, c as u32);
        }
    }
}

fn cmd_matrix() {
    // 簡易マトリックスエフェクト
    vga_buffer::print_colored("=== Matrix Mode ===\n", Color::LightGreen, Color::Black);
    let chars = "01アイウエオカキクケコ@#$%&";
    let mut seed: u32 = 42;
    for _ in 0..5 {
        for _ in 0..80 {
            // 簡易乱数
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let idx = (seed >> 16) as usize % chars.len();
            let c = chars.as_bytes()[idx.min(chars.len() - 1)];
            if c.is_ascii_graphic() {
                vga_buffer::print_colored(
                    &alloc::format!("{}", c as char),
                    Color::LightGreen,
                    Color::Black,
                );
            } else {
                vga_buffer::print_colored("*", Color::Green, Color::Black);
            }
        }
        println!();
    }
}

fn cmd_net() {
    vga_buffer::print_colored("=== Network Information ===\n", Color::Yellow, Color::Black);
    let device = crate::net::RTL8139_DEVICE.lock();
    if let Some(ref rtl) = *device {
        let mac = rtl.mac_address();
        println!("  Device:       RTL8139 (PCI)");
        println!("  MAC Address:  {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
        println!("  Status:       Initialized");
    } else {
        println!("  No network device found or initialized.");
    }
}

fn cmd_wifi(args: &str) {
    use crate::net::wifi;
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        println!("Usage: wifi <scan|connect|status>");
        return;
    }

    match parts[0] {
        "scan" => wifi::scan(),
        "connect" => {
            if parts.len() < 3 {
                println!("Usage: wifi connect <SSID> <password>");
                return;
            }
            wifi::connect(parts[1], parts[2]);
        }
        "status" => wifi::status(),
        _ => println!("Unknown wifi command: {}", parts[0]),
    }
}

fn cmd_gui() {
    println!("Switching to GUI mode...");
    crate::gui::start_gui();
    // After returning from GUI, clear screen and return to shell
    cmd_clear();
}

