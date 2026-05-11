// RustOS - CLIシェル
// キーボード入力を受け取り、コマンドを実行する対話型シェル

use crate::vga_buffer::{self, Color};
use crate::{print, println};
use alloc::string::String;
use alloc::vec::Vec;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

const MAX_HISTORY: usize = 32;
const PROMPT: &str = "rust_os> ";

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
    vga_buffer::print_colored(PROMPT, Color::LightGreen, Color::Black);
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
        "ls" | "dir" => {
            println!("(no filesystem mounted)");
            println!("  /dev/vga    - VGA text mode display");
            println!("  /dev/kbd    - PS/2 keyboard");
            println!("  /dev/serial - Serial port (COM1)");
        }
        "pwd" => println!("/"),
        "cat" => println!("cat: no filesystem available"),
        "" => {}
        _ => {
            vga_buffer::print_colored("Unknown command: ", Color::LightRed, Color::Black);
            println!("'{}'. Type 'help' for available commands.", command);
        }
    }
}

fn cmd_help() {
    vga_buffer::print_colored("=== RustOS Commands ===\n", Color::Yellow, Color::Black);
    let commands = [
        ("help", "Show this help message"),
        ("clear/cls", "Clear the screen"),
        ("echo <text>", "Print text to screen"),
        ("info/sysinfo", "Show system information"),
        ("mem/memory", "Show memory information"),
        ("uptime", "Show system uptime"),
        ("color <fg> [bg]", "Change text color"),
        ("calc <expr>", "Simple calculator (+,-,*,/)"),
        ("history", "Show command history"),
        ("ascii [char]", "Show ASCII table or char code"),
        ("matrix", "Matrix rain effect"),
        ("whoami", "Show current user"),
        ("uname", "Show OS version"),
        ("hostname", "Show hostname"),
        ("ls/dir", "List devices"),
        ("pwd", "Print working directory"),
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
        let allocator = unsafe {
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
    // フォールバック: トリプルフォルト
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
    // ACPI shutdown (Bochs/QEMU)
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

// アロケータの公開参照
pub use crate::allocator::ALLOCATOR;
