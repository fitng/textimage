use clap::Parser;
use crossterm::{
    cursor::{self, Hide, MoveTo},
    event::{self, Event, KeyCode, KeyEventKind},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, Stylize},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use image::{DynamicImage, Rgba, RgbaImage, imageops::FilterType};
use std::error::Error;
use std::fs;
use std::io::{Write, stdout};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Image
    #[arg(short, long)]
    image: String,

    /// Text
    #[arg(short, long)]
    text: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let img = image::open(args.image)?;
    let raw_text = fs::read_to_string(args.text)?;
    let text: String = raw_text.split_whitespace().collect::<Vec<&str>>().join(" ");
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Err("text file is empty".into());
    }

    let _guard = TerminalGuard::new()?;

    let (w, h) = terminal::size()?;
    draw(&img, &chars, w, h)?;

    loop {
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                _ => {}
            },
            Event::Resize(w, h) => draw(&img, &chars, w, h)?,
            _ => {}
        }
    }

    Ok(())
}

fn draw(
    img: &DynamicImage,
    chars: &[char],
    term_w: u16,
    term_h: u16,
) -> Result<(), Box<dyn Error>> {
    const ALPHA_THRESHOLD: u8 = 200;
    const LUMA_THRESHOLD: f32 = 25.0;

    let resized = resize_for_terminal(img, term_w, term_h);
    let (w, h) = resized.dimensions();
    let x_off = term_w.saturating_sub(w as u16) / 2;
    let y_off = term_h.saturating_sub(h as u16) / 2;
    let has_alpha = img.color().has_alpha();

    let mut out = stdout();
    queue!(out, Clear(ClearType::All), SetAttribute(Attribute::Bold))?;

    let mut pointer = 0;
    for (x, y, pixel) in resized.enumerate_pixels() {
        if x == 0 {
            queue!(out, MoveTo(x_off, y_off + y as u16))?;
        }
        let Rgba([r, g, b, a]) = *pixel;
        let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;

        let empty = if has_alpha {
            a < ALPHA_THRESHOLD
        } else {
            luma < LUMA_THRESHOLD
        };

        if empty {
            queue!(out, Print(' '))?;
        } else {
            let ch = chars[pointer % chars.len()];
            queue!(out, Print(ch.with(Color::Rgb { r, g, b })))?;
            pointer += 1;
        }
    }

    queue!(out, SetAttribute(Attribute::Reset))?;
    out.flush()?;
    Ok(())
}

fn resize_for_terminal(img: &DynamicImage, term_w: u16, term_h: u16) -> RgbaImage {
    const CHAR_ASPECT: f32 = 0.45;
    let (w, h) = (img.width() as f32, img.height() as f32);
    let max_w = term_w as f32;
    let max_h = term_h.saturating_sub(1) as f32;

    let scale = (max_w / w).min(max_h / (h * CHAR_ASPECT));
    let new_w = (w * scale).round().max(1.0) as u32;
    let new_h = (h * scale * CHAR_ASPECT).round().max(1.0) as u32;

    img.resize_exact(new_w, new_h, FilterType::Lanczos3)
        .to_rgba8()
}

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> std::io::Result<Self> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
