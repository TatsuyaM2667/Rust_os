use crate::vga_buffer::{self, Color, Writer};
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicBool, Ordering};

lazy_static! {
    static ref MOUSE_POS: Mutex<MousePosition> = Mutex::new(MousePosition { x: 100, y: 100 });
}

pub static GUI_ACTIVE: AtomicBool = AtomicBool::new(false);

struct MousePosition {
    x: i32,
    y: i32,
}

const WIN_X: i32 = 100;
const WIN_Y: i32 = 100;
const WIN_W: i32 = 400;
const WIN_H: i32 = 250;
const BG_COLOR: u32 = 0x223344;
const TASKBAR_HEIGHT: i32 = 40;

pub fn start_gui() {
    GUI_ACTIVE.store(true, Ordering::SeqCst);
    render_desktop();

    loop {
        if !GUI_ACTIVE.load(Ordering::SeqCst) {
            break;
        }
        x86_64::instructions::hlt();
    }
}

pub fn is_active() -> bool {
    GUI_ACTIVE.load(Ordering::SeqCst)
}

pub fn update_mouse_position(dx: i32, dy: i32) {
    if !is_active() { return; }

    let mut pos = MOUSE_POS.lock();
    let old_x = pos.x;
    let old_y = pos.y;

    pos.x = (pos.x + dx).clamp(0, 1270);
    pos.y = (pos.y + dy).clamp(0, 790);

    let new_x = pos.x;
    let new_y = pos.y;

    if old_x != new_x || old_y != new_y {
        draw_cursor_smart(old_x as usize, old_y as usize, new_x as usize, new_y as usize);
    }
}

fn render_desktop() {
    let mut locked_writer = vga_buffer::WRITER.lock();
    let writer = locked_writer.lock();
    let width = writer.info().width;
    let height = writer.info().height;

    // Background
    writer.clear_screen();
    writer.draw_rect(0, 0, width, height, BG_COLOR);

    // Taskbar
    writer.draw_rect(0, height - TASKBAR_HEIGHT as usize, width, TASKBAR_HEIGHT as usize, 0x111111);
    writer.draw_rect(5, height - TASKBAR_HEIGHT as usize + 5, 60, 30, 0x00AAFF);
    draw_string_at(writer, 10, height - TASKBAR_HEIGHT as usize + 12, "RustOS", Color::White as u32, 1);

    // Window
    draw_window(writer);

    // Initial Cursor
    let pos = MOUSE_POS.lock();
    writer.draw_rect(pos.x as usize, pos.y as usize, 10, 10, Color::White as u32);
}

fn draw_window(writer: &mut Writer) {
    writer.draw_rect(WIN_X as usize - 1, WIN_Y as usize - 1, WIN_W as usize + 2, WIN_H as usize + 2, 0xAAAAAA);
    writer.draw_rect(WIN_X as usize, WIN_Y as usize, WIN_W as usize, WIN_H as usize, 0xDDDDDD);
    writer.draw_rect(WIN_X as usize, WIN_Y as usize, WIN_W as usize, 30, 0x334455);
    draw_string_at(writer, WIN_X as usize + 10, WIN_Y as usize + 8, "Welcome to RustOS", Color::White as u32, 1);

    draw_string_at(writer, WIN_X as usize + 20, WIN_Y as usize + 50, "GUI Fixes Applied:", Color::Black as u32, 1);
    draw_string_at(writer, WIN_X as usize + 20, WIN_Y as usize + 80, "- Fixed mouse trails (smart redraw)", Color::DarkGray as u32, 1);
    draw_string_at(writer, WIN_X as usize + 20, WIN_Y as usize + 100, "- Cursor only active in GUI mode", Color::DarkGray as u32, 1);
    draw_string_at(writer, WIN_X as usize + 20, WIN_Y as usize + 120, "- Renamed back to RustOS", Color::DarkGray as u32, 1);

    writer.draw_rect(WIN_X as usize + WIN_W as usize - 30, WIN_Y as usize + 5, 20, 20, 0xAA3333);
    draw_string_at(writer, WIN_X as usize + WIN_W as usize - 23, WIN_Y as usize + 8, "X", Color::White as u32, 1);
}

fn draw_cursor_smart(old_x: usize, old_y: usize, new_x: usize, new_y: usize) {
    let mut locked_writer = vga_buffer::WRITER.lock();
    if let Some(writer) = locked_writer.get_mut() {
        let height = writer.info().height;

        // Restore background at old position by checking what was there
        for dy in 0..10 {
            for dx in 0..10 {
                let px = old_x + dx;
                let py = old_y + dy;
                let color = get_ui_color_at(px as i32, py as i32, height as i32);
                writer.draw_rect(px, py, 1, 1, color);
            }
        }

        // Draw new cursor
        writer.draw_rect(new_x, new_y, 10, 10, Color::White as u32);
    }
}

fn get_ui_color_at(x: i32, y: i32, screen_height: i32) -> u32 {
    // Taskbar
    if y >= screen_height - TASKBAR_HEIGHT {
        if y >= screen_height - TASKBAR_HEIGHT + 5 && y < screen_height - TASKBAR_HEIGHT + 35 && x >= 5 && x < 65 {
            return 0x00AAFF; // Start button
        }
        return 0x111111;
    }
    // Window
    if x >= WIN_X && x < WIN_X + WIN_W && y >= WIN_Y && y < WIN_Y + WIN_H {
        if y < WIN_Y + 30 {
            return 0x334455; // Title bar
        }
        return 0xDDDDDD; // Client area
    }
    // Window border
    if x >= WIN_X - 1 && x < WIN_X + WIN_W + 1 && y >= WIN_Y - 1 && y < WIN_Y + WIN_H + 1 {
        return 0xAAAAAA;
    }
    
    BG_COLOR
}

fn draw_string_at(writer: &mut vga_buffer::Writer, x: usize, y: usize, s: &str, color: u32, scale: usize) {
    let mut current_x = x;
    for c in s.chars() {
        writer.draw_char_at(current_x, y, c, color, scale);
        current_x += 8 * scale;
    }
}
