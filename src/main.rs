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

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

// bootloader クレートのエントリポイントマクロ
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // === Phase 1: 基本初期化 ===
    // GDTの設定
    gdt::init();

    // IDT (Interrupt Descriptor Table) の設定
    interrupts::init_idt();

    // PIC (Programmable Interrupt Controller) の初期化
    unsafe {
        interrupts::PICS.lock().initialize();
    }

    // === Phase 2: メモリ管理の初期化 ===
    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // ヒープの初期化
    memory::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");
    allocator::init_allocator(memory::HEAP_START, memory::HEAP_SIZE);

    // === Phase 3: ネットワーク初期化 ===
    net::init(&mut mapper, &mut frame_allocator, phys_mem_offset);

    // === Phase 4: 割り込み有効化 ===
    x86_64::instructions::interrupts::enable();

    // === Phase 4: シェルの起動 ===
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
