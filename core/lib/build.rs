//! Ensures Rocket isn't compiled with an incompatible version of Rust.

use yansi::{Paint, Color::{Red, Yellow, Blue}};

// Specifies the minimum nightly version needed to compile Rocket.
const MIN_DATE: &'static str = "2019-04-05";
const MIN_VERSION: &'static str = "1.35.0-nightly";

macro_rules! err {
    ($version:expr, $date:expr, $msg:expr) => (
        eprintln!("{} {}", Red.paint("Error:").bold(), Paint::new($msg).bold());
        eprintln!("Installed version: {}", Yellow.paint(format!("{} ({})", $version, $date)));
        eprintln!("Minimum required:  {}", Yellow.paint(format!("{} ({})", MIN_VERSION, MIN_DATE)));
    )
}

fn main() {
    if let Some((version, channel, date)) = version_check::triple() {
        if !channel.supports_features() {
            err!(version, date, "Rocket (core) requires a 'dev' or 'nightly' version of rustc.");

            eprint!("{}", Blue.paint("See the getting started guide ("));
            eprint!("https://rocket.rs/v0.5/guide/getting-started/");
            eprintln!("{}", Blue.paint(") for more information."));

            panic!("Aborting compilation due to incompatible compiler.")
        }

        if !version.at_least(MIN_VERSION) || !date.at_least(MIN_DATE) {
            err!(version, date, "Rocket (core) requires a more recent version of rustc.");
            panic!("Aborting compilation due to incompatible compiler.")
        }
    } else {
        println!("cargo:warning={}", "Rocket was unable to check rustc compiler compatibility.");
        println!("cargo:warning={}", "Build may fail due to incompatible rustc version.");
    }
}
