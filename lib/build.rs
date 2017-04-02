//! This tiny build script ensures that rocket is not compiled with an
//! incompatible version of rust.

extern crate ansi_term;
extern crate version_check;

use ansi_term::Color::{Red, Yellow, Blue, White};
use version_check::{is_nightly, is_min_version};

// Specifies the minimum nightly version needed to compile Rocket.
const MIN_VERSION: &'static str = "1.16.0-nightly";

// Convenience macro for writing to stderr.
macro_rules! printerr {
    ($($arg:tt)*) => ({
        use std::io::prelude::*;
        write!(&mut ::std::io::stderr(), "{}\n", format_args!($($arg)*))
            .expect("Failed to write to stderr.")
    })
}

fn main() {
    let (ok_nightly, ok_version) = (is_nightly(), is_min_version(MIN_VERSION));
    let print_version_err = |version: &str| {
        printerr!("{} {}. {} {}.",
                  White.paint("Installed version is:"),
                  Yellow.paint(version),
                  White.paint("Minimum required:"),
                  Yellow.paint(MIN_VERSION));
    };

    if let (Some(is_nightly), Some((ok_version, version))) = (ok_nightly, ok_version) {
        if !is_nightly {
            printerr!("{} {}",
                      Red.bold().paint("Error:"),
                      White.paint("Rocket requires a nightly version of Rust."));
            print_version_err(&*version);
            printerr!("{}{}{}",
                      Blue.paint("See the getting started guide ("),
                      White.paint("https://rocket.rs/guide/getting-started/"),
                      Blue.paint(") for more information."));
            panic!("Aborting compilation due to incompatible compiler.")
        }

        if !ok_version {
            printerr!("{} {}",
                      Red.bold().paint("Error:"),
                      White.paint("Rocket requires a newer version of rustc."));
            printerr!("{}{}{}",
                      Blue.paint("Use `"),
                      White.paint("rustup update"),
                      Blue.paint("` or your preferred method to update Rust."));
            print_version_err(&*version);
            panic!("Aborting compilation due to incompatible compiler.")
        }
    } else {
        println!("cargo:warning={}", "Rocket was unable to check rustc compatibility.");
        println!("cargo:warning={}", "Build may fail due to incompatible rustc version.");
    }
}
