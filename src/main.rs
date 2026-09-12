use std::path::PathBuf;
use std::time::Duration;

mod command;
mod overlay;
mod socket;
mod sprite;
mod state_machine;

use overlay::{run_overlay, OverlayConfig};
use sprite::Sprite;
use state_machine::AnimationConfig;

fn print_usage() {
    eprintln!(
        "Usage: felt-overlay [OPTIONS]\n\n\
         Options:\n\
           --image <path>       Custom PNG image for companion sprite\n\
           --socket <path>      Custom Unix socket path (default: $XDG_RUNTIME_DIR/felt-overlay.sock)\n\
           --timeout <secs>     Auto-hide duration in seconds (default: 5.0)\n\
           --slide <millis>     Slide animation duration in milliseconds (default: 250)\n\
           --help, -h           Show this help message\n"
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut custom_image: Option<PathBuf> = None;
    let mut custom_socket: Option<PathBuf> = None;
    let mut auto_hide_secs: f32 = 5.0;
    let mut slide_millis: u64 = 250;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--image" => {
                if let Some(val) = args.next() {
                    custom_image = Some(PathBuf::from(val));
                } else {
                    eprintln!("Error: --image requires a file path");
                    std::process::exit(1);
                }
            }
            "--socket" => {
                if let Some(val) = args.next() {
                    custom_socket = Some(PathBuf::from(val));
                } else {
                    eprintln!("Error: --socket requires a file path");
                    std::process::exit(1);
                }
            }
            "--timeout" => {
                if let Some(val) = args.next() {
                    if let Ok(secs) = val.parse::<f32>() {
                        auto_hide_secs = secs;
                    } else {
                        eprintln!("Error: invalid timeout value: {}", val);
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("Error: --timeout requires seconds");
                    std::process::exit(1);
                }
            }
            "--slide" => {
                if let Some(val) = args.next() {
                    if let Ok(ms) = val.parse::<u64>() {
                        slide_millis = ms;
                    } else {
                        eprintln!("Error: invalid slide value: {}", val);
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("Error: --slide requires milliseconds");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown option: {}", other);
                print_usage();
                std::process::exit(1);
            }
        }
    }

    let sprite = match custom_image {
        Some(path) => match Sprite::load_from_file(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to load sprite from {:?}: {}", path, e);
                std::process::exit(1);
            }
        },
        None => match Sprite::load_default() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to load embedded sprite: {}", e);
                std::process::exit(1);
            }
        },
    };

    let animation = AnimationConfig {
        hidden_y: 256.0,
        resting_y: 32.0,
        slide_duration: Duration::from_millis(slide_millis),
        auto_hide_duration: Duration::from_secs_f32(auto_hide_secs),
    };

    let config = OverlayConfig {
        surface_width: 256,
        surface_height: 256,
        margin_bottom: 16,
        margin_right: 16,
        animation,
    };

    if let Err(e) = run_overlay(sprite, config, custom_socket) {
        eprintln!("felt-overlay error: {}", e);
        std::process::exit(1);
    }
}
