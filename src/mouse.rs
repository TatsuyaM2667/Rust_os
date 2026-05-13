use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::gui;

lazy_static! {
    static ref MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState::new());
}

struct MouseState {
    phase: u8,
    buffer: [u8; 3],
}

impl MouseState {
    fn new() -> Self {
        MouseState {
            phase: 0,
            buffer: [0; 3],
        }
    }
}

pub fn init() {
    unsafe {
        let mut cmd_port = Port::<u8>::new(0x64);
        let mut data_port = Port::<u8>::new(0x60);

        // Enable Mouse (Auxiliary Device)
        wait_for_write();
        cmd_port.write(0xA8);

        // Enable Interrupts in Controller Configuration
        wait_for_write();
        cmd_port.write(0x20); // Read Config
        wait_for_read();
        let mut config = data_port.read();
        config |= 0x01; // Enable IRQ 1 (Keyboard)
        config |= 0x02; // Enable IRQ 12 (Mouse)
        config &= !0x10; // Enable Keyboard Clock (Bit 4 is disable, so clear it)
        config &= !0x20; // Enable Mouse Clock (Bit 5 is disable, so clear it)
        wait_for_write();
        cmd_port.write(0x60); // Write Config
        wait_for_write();
        data_port.write(config);

        // Tell Mouse to use default settings
        mouse_write(0xF6);
        let _ = mouse_read(); // ACK

        // Enable Data Reporting
        mouse_write(0xF4);
        let _ = mouse_read(); // ACK
    }
}

fn wait_for_read() {
    let mut port = Port::<u8>::new(0x64);
    while (unsafe { port.read() } & 1) == 0 {}
}

fn wait_for_write() {
    let mut port = Port::<u8>::new(0x64);
    while (unsafe { port.read() } & 2) != 0 {}
}

fn mouse_write(data: u8) {
    let mut cmd_port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);
    wait_for_write();
    unsafe { cmd_port.write(0xD4); } // Write to Mouse
    wait_for_write();
    unsafe { data_port.write(data); }
}

fn mouse_read() -> u8 {
    let mut data_port = Port::<u8>::new(0x60);
    wait_for_read();
    unsafe { data_port.read() }
}

pub fn handle_interrupt(data: u8) {
    let mut state = MOUSE_STATE.lock();
    match state.phase {
        0 => {
            // First byte: flags (bit 3 should be 1 for sync)
            if (data & 0x08) != 0 {
                state.buffer[0] = data;
                state.phase = 1;
            }
        }
        1 => {
            // Second byte: X movement
            state.buffer[1] = data;
            state.phase = 2;
        }
        2 => {
            // Third byte: Y movement
            state.buffer[2] = data;
            state.phase = 0;

            let flags = state.buffer[0];
            let x_move = state.buffer[1] as i32;
            let y_move = state.buffer[2] as i32;

            // Handle signs (X and Y are relative movements)
            let x = if (flags & 0x10) != 0 { x_move - 256 } else { x_move };
            let y = if (flags & 0x20) != 0 { y_move - 256 } else { y_move };

            // Invert Y because mouse coordinate system is often inverted relative to framebuffer
            gui::update_mouse_position(x, -y);
        }
        _ => state.phase = 0,
    }
}
