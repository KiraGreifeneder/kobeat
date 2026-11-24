use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use evdev::{AbsoluteAxisCode, Device, EventSummary, KeyCode};
use framebuffer::{Bitfield, Framebuffer, VarScreeninfo};

use crate::client::client;
use crate::domain::player_command::{PlayerCommand, PlayerResponse};
use crate::domain::state::{PlaybackState, PlayerState};

mod font;

const FB_DEVICE: &str = "/dev/fb0";
const REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const FG: (u8, u8, u8) = (0, 0, 0);
const BG: (u8, u8, u8) = (255, 255, 255);
const ACCENT: (u8, u8, u8) = (142, 234, 96);

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Rect {
    fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Entry point for the framebuffer-native UI, replacing kobeatui for
/// devices where the built-in browser can't render a usable interface.
/// Draws directly to /dev/fb0 and reads taps from the touchscreen's evdev
/// device instead of serving HTML.
pub fn run() -> Result<()> {
    let mut fb = Framebuffer::new(FB_DEVICE).map_err(|e| anyhow!("failed to open {FB_DEVICE}: {e}"))?;
    let width = fb.var_screen_info.xres;
    let height = fb.var_screen_info.yres;

    let taps = spawn_touch_reader()?;

    let progress_bar = Rect { x: 0, y: 0, w: width, h: height / 10 };
    let button_h = height / 6;
    let button_y = height - button_h;
    let button_w = width / 3;
    let prev_btn = Rect { x: 0, y: button_y, w: button_w, h: button_h };
    let play_btn = Rect { x: button_w, y: button_y, w: button_w, h: button_h };
    let next_btn = Rect { x: button_w * 2, y: button_y, w: width - button_w * 2, h: button_h };

    loop {
        while let Ok((x, y)) = taps.try_recv() {
            if progress_bar.contains(x, y) {
                if let Ok(PlayerResponse::State(state)) = send(PlayerCommand::GetPlayerState) {
                    if let Some(track) = state.current_track {
                        let percent = x as f64 / progress_bar.w.max(1) as f64;
                        let _ = send(PlayerCommand::Seek {
                            position_secs: (percent * track.duration_secs) as u64,
                        });
                    }
                }
            } else if prev_btn.contains(x, y) {
                let _ = send(PlayerCommand::PlayPrevious);
            } else if next_btn.contains(x, y) {
                let _ = send(PlayerCommand::PlayNext);
            } else if play_btn.contains(x, y) {
                let playing = matches!(
                    send(PlayerCommand::GetPlayerState),
                    Ok(PlayerResponse::State(PlayerState { playback_state: PlaybackState::Playing, .. }))
                );
                let _ = send(if playing { PlayerCommand::Pause } else { PlayerCommand::Resume });
            }
        }

        let state = match send(PlayerCommand::GetPlayerState) {
            Ok(PlayerResponse::State(state)) => Some(state),
            _ => None,
        };

        draw(&mut fb, width, height, progress_bar, prev_btn, play_btn, next_btn, state.as_ref());

        std::thread::sleep(REFRESH_INTERVAL);
    }
}

fn send(command: PlayerCommand) -> Result<PlayerResponse> {
    let raw = client::send_command(&command)?;
    serde_json::from_str(&raw).context("failed to parse daemon response")
}

fn draw(
    fb: &mut Framebuffer,
    width: u32,
    height: u32,
    progress_bar: Rect,
    prev_btn: Rect,
    play_btn: Rect,
    next_btn: Rect,
    state: Option<&PlayerState>,
) {
    fill_rect(fb, Rect { x: 0, y: 0, w: width, h: height }, BG);

    let (position, duration, playing) = state
        .map(|s| {
            (
                s.base_position_secs,
                s.current_track.as_ref().map(|t| t.duration_secs).unwrap_or(0.0),
                matches!(s.playback_state, PlaybackState::Playing),
            )
        })
        .unwrap_or((0.0, 0.0, false));

    let percent = if duration > 0.0 { (position / duration).clamp(0.0, 1.0) } else { 0.0 };
    fill_rect(fb, Rect { x: 0, y: 0, w: (progress_bar.w as f64 * percent) as u32, h: progress_bar.h }, ACCENT);

    let time_label = format!("{}/{}", format_time(position), format_time(duration));
    draw_text(fb, 8, progress_bar.h + 8, &time_label, 6, FG);

    fill_rect(fb, Rect { x: 0, y: prev_btn.y, w: width, h: prev_btn.h }, ACCENT);
    fill_triangle_left(fb, inset(prev_btn), FG);
    if playing {
        draw_pause_icon(fb, inset(play_btn), FG);
    } else {
        fill_triangle_right(fb, inset(play_btn), FG);
    }
    fill_triangle_right(fb, inset(next_btn), FG);
}

fn inset(rect: Rect) -> Rect {
    let margin = rect.h / 4;
    Rect { x: rect.x + margin, y: rect.y + margin, w: rect.w.saturating_sub(margin * 2), h: rect.h.saturating_sub(margin * 2) }
}

fn format_time(total_secs: f64) -> String {
    let total = total_secs.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn draw_text(fb: &mut Framebuffer, x: u32, y: u32, text: &str, scale: u32, color: (u8, u8, u8)) {
    let mut cursor_x = x;
    for ch in text.chars() {
        for (row, bits) in font::glyph_rows(ch).iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    fill_rect(
                        fb,
                        Rect { x: cursor_x + col * scale, y: y + row as u32 * scale, w: scale, h: scale },
                        color,
                    );
                }
            }
        }
        cursor_x += 4 * scale;
    }
}

fn fill_triangle_right(fb: &mut Framebuffer, rect: Rect, color: (u8, u8, u8)) {
    let half = (rect.h / 2).max(1);
    for dy in 0..rect.h {
        let dist = (dy as i32 - half as i32).unsigned_abs();
        let w = rect.w.saturating_sub(dist * rect.w / half);
        fill_rect(fb, Rect { x: rect.x, y: rect.y + dy, w, h: 1 }, color);
    }
}

fn fill_triangle_left(fb: &mut Framebuffer, rect: Rect, color: (u8, u8, u8)) {
    let half = (rect.h / 2).max(1);
    for dy in 0..rect.h {
        let dist = (dy as i32 - half as i32).unsigned_abs();
        let w = rect.w.saturating_sub(dist * rect.w / half);
        fill_rect(fb, Rect { x: rect.x + rect.w - w, y: rect.y + dy, w, h: 1 }, color);
    }
}

fn draw_pause_icon(fb: &mut Framebuffer, rect: Rect, color: (u8, u8, u8)) {
    let bar_w = (rect.w / 5).max(1);
    fill_rect(fb, Rect { x: rect.x + bar_w, y: rect.y, w: bar_w, h: rect.h }, color);
    fill_rect(fb, Rect { x: rect.x + rect.w.saturating_sub(2 * bar_w), y: rect.y, w: bar_w, h: rect.h }, color);
}

fn fill_rect(fb: &mut Framebuffer, rect: Rect, color: (u8, u8, u8)) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            put_pixel(fb, x, y, color);
        }
    }
}

fn put_pixel(fb: &mut Framebuffer, x: u32, y: u32, color: (u8, u8, u8)) {
    let bpp = (fb.var_screen_info.bits_per_pixel / 8) as usize;
    let line_length = fb.fix_screen_info.line_length as usize;
    let offset = y as usize * line_length + x as usize * bpp;
    if bpp == 0 || offset + bpp > fb.frame.len() {
        return;
    }
    let packed = pack_color(&fb.var_screen_info, color).to_le_bytes();
    fb.frame[offset..offset + bpp].copy_from_slice(&packed[..bpp]);
}

fn pack_color(var: &VarScreeninfo, (r, g, b): (u8, u8, u8)) -> u32 {
    let pack = |value: u8, field: &Bitfield| -> u32 {
        if field.length == 0 {
            return 0;
        }
        let scaled = (value as u32 * ((1u32 << field.length) - 1)) / 255;
        scaled << field.offset
    };
    pack(r, &var.red) | pack(g, &var.green) | pack(b, &var.blue)
}

/// Reads touch taps on a background thread and forwards each tap's
/// coordinates once the finger lifts, so the main loop never blocks on
/// input while still redrawing on a timer.
fn spawn_touch_reader() -> Result<mpsc::Receiver<(u32, u32)>> {
    let mut device = find_touch_device()?;
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut x, mut y) = (0i32, 0i32);
        let mut touching = false;
        loop {
            let Ok(events) = device.fetch_events() else { break };
            for event in events {
                match event.destructure() {
                    EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_X, v)
                    | EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_X, v) => x = v,
                    EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_Y, v)
                    | EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_Y, v) => y = v,
                    EventSummary::Key(_, KeyCode::BTN_TOUCH, 1) => touching = true,
                    EventSummary::Key(_, KeyCode::BTN_TOUCH, 0) => {
                        if touching {
                            let _ = tx.send((x.max(0) as u32, y.max(0) as u32));
                        }
                        touching = false;
                    }
                    _ => {}
                }
            }
        }
    });

    Ok(rx)
}

fn find_touch_device() -> Result<Device> {
    for entry in std::fs::read_dir("/dev/input").context("failed to list /dev/input")? {
        let path = entry?.path();
        if let Ok(device) = Device::open(&path) {
            let is_touch = device
                .supported_absolute_axes()
                .is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_X) || axes.contains(AbsoluteAxisCode::ABS_X));
            if is_touch {
                return Ok(device);
            }
        }
    }
    bail!("no touchscreen device found under /dev/input")
}
