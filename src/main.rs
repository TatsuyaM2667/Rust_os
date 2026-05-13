#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
mod vga_buffer;
mod allocator;
mod fs;
mod gdt;
mod gui;
mod interrupts;
mod memory;
mod net;
mod pacman;
mod pci;
mod serial;
mod shell;

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;

// bootloader_api の設定
pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

// エントリポイント
entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // === Phase 0: コンソールの初期化 ===
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let info = framebuffer.info();
        let buffer = framebuffer.buffer_mut();
        vga_buffer::WRITER.lock().init(buffer, info);
    }

    // シリアル出力で起動を確認
    serial_println!("Kernel started!");

    let phys_mem_offset_val = boot_info.physical_memory_offset.into_option().expect("Physical memory offset not found");
    serial_println!("Physical memory offset: 0x{:x}", phys_mem_offset_val);

    // スタックのアドレスを確認
    let stack_ptr: u64;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) stack_ptr);
    }
    serial_println!("Current RSP: 0x{:x}", stack_ptr);

    // === Phase 1: 基本初期化 ===
    // GDTの設定
    serial_println!("Initializing GDT...");
    gdt::init();

    // IDT (Interrupt Descriptor Table) の設定
    serial_println!("Initializing IDT...");
    interrupts::init_idt();

    // PIC (Programmable Interrupt Controller) の初期化
    serial_println!("Initializing PIC...");
    unsafe {
        let mut pics = interrupts::PICS.lock();
        // すべての割り込みを一度マスクする
        pics.write_masks(0xFF, 0xFF);
        pics.initialize();
        // IRQ 0 (タイマー) と IRQ 1 (キーボード) のマスクを解除
        pics.write_masks(0xFC, 0xFF);
    }

    // PS/2キーボードの初期化 (Port 0x64/0x60)
    serial_println!("Initializing PS/2 Keyboard...");
    unsafe {
        use x86_64::instructions::port::Port;
        let mut cmd_port = Port::<u8>::new(0x64);
        let mut data_port = Port::<u8>::new(0x60);
        
        // 0xAE: キーボードインタフェースの有効化
        cmd_port.write(0xAE);
        
        // ステータスレジスタを確認しながら入力バッファを空にする
        for _ in 0..1000 {
            let status = cmd_port.read();
            if (status & 0x01) != 0 {
                data_port.read();
            }
        }
        
        // 0xF4: キーボードのスキャン開始（コマンド）
        data_port.write(0xF4);
    }

    // === Phase 2: メモリ管理の初期化 ===
    serial_println!("Initializing Memory Management...");
    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    // ヒープの初期化
    serial_println!("Initializing Heap...");
    memory::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");
    allocator::init_allocator(memory::HEAP_START, memory::HEAP_SIZE);

    // === Phase 3: ネットワーク初期化 ===
    serial_println!("Initializing Network...");
    net::init(&mut mapper, &mut frame_allocator, phys_mem_offset);

    // === Phase 4: 割り込み有効化 ===
    serial_println!("Enabling Interrupts...");
    x86_64::instructions::interrupts::enable();

    // === Phase 4: シェルの起動 ===
    serial_println!("Starting Shell...");
    shell::init();

    // メインループ: halt命令で CPU を省電力モードに
    loop {
        x86_64::instructions::hlt();
    }
}

/// パニックハンドラ
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // エラーを赤で表示
    vga_buffer::print_colored(
        "\n!!! KERNEL PANIC !!!\n",
        vga_buffer::Color::White,
        vga_buffer::Color::Red,
    );
    println!("{}", info);

    // シリアルにも出力
    serial_println!("KERNEL PANIC: {}", info);

    loop {
        x86_64::instructions::hlt();
    }
}

/// ヒープ割り当て失敗時のハンドラ
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
