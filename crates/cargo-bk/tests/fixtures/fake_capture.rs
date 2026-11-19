//! A stand-in for a product's capture command, compiled by tests/capture.rs.
//! Usage: fake_capture --log=<file> --screen=<name> --capture=<path>
//! The screen name picks the behaviour.

use std::io::Write;

const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xff, 0xff, 0x3f,
    0x00, 0x05, 0xfe, 0x02, 0xfe, 0xa7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
    0x44, 0xae, 0x42, 0x60, 0x82,
];

fn main() {
    let mut log = None;
    let mut screen = String::new();
    let mut capture = String::new();
    let mut fail_build = false;
    for a in std::env::args().skip(1) {
        if let Some(v) = a.strip_prefix("--log=") {
            log = Some(v.to_string());
        } else if let Some(v) = a.strip_prefix("--screen=") {
            screen = v.to_string();
        } else if a == "--build-fail" {
            fail_build = true;
        } else if let Some(v) = a.strip_prefix("--capture=") {
            capture = v.to_string();
        }
    }
    let building = capture.is_empty();
    if let Some(log) = log {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .unwrap();
        if building {
            writeln!(f, "BUILD").unwrap();
        } else {
            let name = std::path::Path::new(&capture)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            writeln!(
                f,
                "{screen} {name} {}",
                std::path::Path::new(&capture).is_absolute()
            )
            .unwrap();
        }
    }
    if building {
        if fail_build {
            eprintln!("build exploded");
        }
        std::process::exit(i32::from(fail_build));
    }
    match screen.as_str() {
        "fail" => {
            eprintln!("capture failed: boom");
            std::process::exit(1);
        }
        "hang" => loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        },
        "empty" => {}
        "partial" => {
            std::fs::write(&capture, b"half").unwrap();
            std::process::exit(1);
        }
        _ => std::fs::write(&capture, PNG).unwrap(),
    }
}
