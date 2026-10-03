//! M0 spike 3 (manual): dump the key / mouse events crossterm reports in the current terminal.
//!
//! Run `cargo run --example keydump` in kitty, foot, alacritty, wezterm, GNOME Terminal,
//! tmux, iTerm2, Terminal.app and try: Shift/Ctrl/Alt + arrows, Ctrl/Shift + PgUp/PgDn,
//! Ctrl+S, Ctrl+Q, Esc, mouse click / wheel. Press `q` twice to quit.
use std::io::{Write, stdout};

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement};

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let enhanced = supports_keyboard_enhancement().unwrap_or(false);
    let mut out = stdout();
    execute!(out, EnableMouseCapture)?;
    if enhanced {
        execute!(
            out,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        )?;
    }
    write!(
        out,
        "keyboard enhancement supported: {enhanced}\r\npress q twice to quit\r\n"
    )?;
    out.flush()?;
    let mut quits = 0;
    loop {
        let ev = event::read()?;
        write!(out, "{ev:?}\r\n")?;
        out.flush()?;
        if let Event::Key(k) = &ev
            && k.kind == KeyEventKind::Press
            && k.code == KeyCode::Char('q')
        {
            quits += 1;
            if quits == 2 {
                break;
            }
        }
    }
    if enhanced {
        execute!(out, PopKeyboardEnhancementFlags)?;
    }
    execute!(out, DisableMouseCapture)?;
    disable_raw_mode()
}
