use x86_64::instructions::port::Port;

const BGA_INDEX_PORT: u16 = 0x1CE;
const BGA_DATA_PORT: u16 = 0x1CF;

const BGA_INDEX_ID: u16 = 0;
const BGA_INDEX_XRES: u16 = 1;
const BGA_INDEX_YRES: u16 = 2;
const BGA_INDEX_BPP: u16 = 3;
const BGA_INDEX_ENABLE: u16 = 4;

const BGA_DISABLED: u16 = 0x00;
const BGA_ENABLED: u16 = 0x01;
const BGA_LFB_ENABLED: u16 = 0x40;

pub struct Bga {
    pub width: u16,
    pub height: u16,
    pub bpp: u16,
    pub framebuffer: *mut u32,
}

impl Bga {
    fn write(index: u16, data: u16) {
        unsafe {
            Port::new(BGA_INDEX_PORT).write(index);
            Port::new(BGA_DATA_PORT).write(data);
        }
    }

    pub fn new() -> Option<Self> {
        // Detect BGA
        unsafe {
            Port::new(BGA_INDEX_PORT).write(BGA_INDEX_ID);
            let id = Port::<u16>::new(BGA_DATA_PORT).read();
            if id < 0xB0C0 { return None; }
        }

        Some(Bga {
            width: 800,
            height: 600,
            bpp: 32,
            // Standard QEMU LFB address. In a real OS, we'd get this from PCI BAR0.
            framebuffer: 0xFD000000 as *mut u32, 
        })
    }

    pub fn set_video_mode(&self, enabled: bool) {
        if enabled {
            Self::write(BGA_INDEX_ENABLE, BGA_DISABLED);
            Self::write(BGA_INDEX_XRES, self.width);
            Self::write(BGA_INDEX_YRES, self.height);
            Self::write(BGA_INDEX_BPP, self.bpp);
            Self::write(BGA_INDEX_ENABLE, BGA_ENABLED | BGA_LFB_ENABLED);
        } else {
            Self::write(BGA_INDEX_ENABLE, BGA_DISABLED);
        }
    }

    pub fn draw_pixel(&self, x: u16, y: u16, color: u32) {
        if x < self.width && y < self.height {
            let offset = (y as usize * self.width as usize) + x as usize;
            unsafe {
                *self.framebuffer.add(offset) = color;
            }
        }
    }

    pub fn clear(&self, color: u32) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.draw_pixel(x, y, color);
            }
        }
    }
}

pub fn start_gui() {
    if let Some(bga) = Bga::new() {
        bga.set_video_mode(true);
        bga.clear(0x00336699); // Dark blue background

        // Draw a simple "Window"
        draw_rect(&bga, 100, 100, 400, 300, 0x00C0C0C0); // Gray window
        draw_rect(&bga, 100, 100, 400, 30, 0x00000080);  // Blue title bar

        // Wait for a "key" to return (simulated)
        // In a real OS, we'd have a GUI loop here.
        for _ in 0..50000000 { unsafe { core::arch::asm!("nop"); } }

        bga.set_video_mode(false);
    } else {
        crate::println!("BGA (Graphics) not supported.");
    }
}

fn draw_rect(bga: &Bga, x: u16, y: u16, w: u16, h: u16, color: u32) {
    for dy in 0..h {
        for dx in 0..w {
            bga.draw_pixel(x + dx, y + dy, color);
        }
    }
}
