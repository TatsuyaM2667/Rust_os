// RustOS - 割り込み処理 (IDT, PIC, キーボード, タイマー)
use crate::gdt;
use crate::shell;
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

// 割り込み番号
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    Network = PIC_1_OFFSET + 11, // Standard IRQ 11 for PCI
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// タイマーティック数
pub static TICKS: Mutex<u64> = Mutex::new(0);

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Network.as_usize()].set_handler_fn(network_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

// ブレークポイント例外
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// ダブルフォルト例外
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

// タイマー割り込み
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut ticks = TICKS.lock();
    *ticks += 1;

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

// キーボード割り込み
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // シェルにスキャンコードを送る
    shell::handle_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn network_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut device = crate::net::RTL8139_DEVICE.lock();
    if let Some(ref mut rtl) = *device {
        rtl.handle_interrupt();
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Network.as_u8());
    }
}

pub fn get_uptime_seconds() -> u64 {
    // PIT はデフォルトで約 18.2Hz
    let ticks = *TICKS.lock();
    ticks / 18
}
