//! Ensures Rocket isn't compiled with an incompatible version of Rust.

use yansi::{Paint, Color::{Red, Yellow}};

const MIN_VERSION: &'static str = "1.45.0";

macro_rules! err {
    ($version:expr, $msg:expr) => (
        eprintln!("{} {}", Red.paint("Error:").bold(), Paint::new($msg).bold());
        eprintln!("Installed version: {}", Yellow.paint(format!("{}", $version)));
        eprintln!("Minimum required:  {}", Yellow.paint(format!("{}", MIN_VERSION)));
    )
}

fn main() {
    if let Some(version) = version_check::Version::read() {
        if !version.at_least(MIN_VERSION) {
            err!(version, "Rocket requires a more recent version of rustc.");
            panic!("Aborting compilation due to incompatible compiler.")
        }
    } else {
        println!("cargo:warning={}", "Rocket was unable to check rustc compiler compatibility.");
        println!("cargo:warning={}", "Build may fail due to incompatible rustc version.");
    }
}
