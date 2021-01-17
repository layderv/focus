extern crate scrap;
extern crate minifb;

use clap::Clap;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;

#[derive(Debug)]
struct Coords(usize, usize);

impl Coords {
    pub fn w(&self) -> usize { self.0 }
    pub fn h(&self) -> usize { self.1 }
}

fn parse_pair(s: &str) -> Result<Coords, &'static str> {
    let split: Vec<String> = s
        .replace("(", "").replace(")", "")
        .split(",")
        .map(|x| x.trim().to_owned())
        .collect();
    match split.len() {
        1 | 2 => Ok(Coords(split[0].parse::<usize>().unwrap(), split[split.len() - 1].parse::<usize>()
            .expect("number must be a positive integer"))),
        _ => Err("valid sizes: 1 and 2")
    }
}

#[derive(Clap, Debug)]
#[clap(name="Focus", version = "1.0", author = "layderv",
    about = "Re-draw and stretch a part of a display to a window")]
struct Opts {
    #[clap(short, long,
        about = "The display to capture. By default, the primary display. The number is zero-based")]
    display: Option<usize>,

    #[clap(short = 'w', long = "window_size", parse(try_from_str = parse_pair),
        about = "The initial size of the new window. Format: (width, height)")]
    window_size: Option<Coords>,

    #[clap(short = 'f', long = "focus_size", parse(try_from_str = parse_pair),
        about = "The rectangle in the display to capture. Default to the whole screen. Format: (width, height)")]
    focus_size: Option<Coords>,

    #[clap(short = 'c', long = "focus_coords", parse(try_from_str = parse_pair),
        about = "Coordinates of the first pixel of `display` to capture. Format: (width, height). Default to (0, 0)")]
    focus_coords: Option<Coords>,

    #[clap(short = 's', long = "stretch",
        about = "Scale mode: AspectRatioStretch by default, fully Stretch otherwise")]
    stretch: bool,
}

fn main() {
    let opts: Opts = Opts::parse();

    let d = if let Some(display) = opts.display {
        let mut displays = Display::all().expect("no displays have been found");
        if displays.len() <= display {
            panic!("display not found")
        }
        displays.remove(display)
    } else {
        Display::primary().expect("no primary display found")
    };

    let (w, h) = (d.width(), d.height());
    let window_size = opts.window_size.unwrap_or(Coords(w, h));
    let focus = opts.focus_size.unwrap_or(Coords(w, h));
    let start = opts.focus_coords.unwrap_or(Coords(0, 0));

    if start.w() + focus.w() > w || start.h() + focus.h() > h {
        panic!("The box must be within the screen. Screen size: {}x{}", w, h)
    }

    let scale_mode = if opts.stretch { ScaleMode::Stretch } else { ScaleMode::AspectRatioStretch };

    let mut window = Window::new(
        "Focus - ESC to exit",
        window_size.w(),
        window_size.h(),
        WindowOptions {
            resize: true,
            scale_mode,
            ..WindowOptions::default()
        },
    )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });

    window.limit_update_rate(Some(std::time::Duration::from_micros(8300))); // 30fps

    let mut capturer = Capturer::new(d).unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        match capturer.frame() {
            Ok(frame) => {
                let stride = frame.len() / h;
                let mut u32_buffer: Vec<u32> = Vec::new();
                for y in start.h()..(start.h() + focus.h()) { // h
                    for x in start.w()..(start.w() + focus.w()) { // w
                        let i = stride * y + 4 * x;
                        if i + 2 < frame.len() {
                            u32_buffer.extend_from_slice(&[
                                (frame[i + 2] as u32) << 16 |
                                (frame[i + 1] as u32) << 8 |
                                frame[i] as u32
                            ]);
                        }
                    }
                }
                match window.update_with_buffer(&u32_buffer, focus.w(), focus.h()) {
                    Err(e) => panic!(e),
                    _ => ()
                }
            }
            Err(ref e) if e.kind() == WouldBlock => {
                // Wait for the frame.
            }
            Err(_) => {
                // We're done here.
                break;
            }
        }
    }
}
