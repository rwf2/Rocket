#![feature(slice_patterns)]

//! This tiny build script ensures that rocket_codegen is not compiled with an
//! incompatible version of rust. It does this by executing `rustc --version`
//! and comparing the version to `MIN_VERSION`, the minimum required version. If
//! the installed version is less than the minimum required version, an error is
//! printed out to the console and compilation is halted.

extern crate ansi_term;

use std::env;
use std::process::Command;

use ansi_term::Colour::{Red, Yellow, Blue, White};

// Specifies the minimum nightly version needed to compile Rocket's codegen.
const MIN_VERSION: &'static str = "2017-01-03";

// Convenience macro for writing to stderr.
macro_rules! printerr {
    ($($arg:tt)*) => ({
        use std::io::prelude::*;
        write!(&mut ::std::io::stderr(), "{}\n", format_args!($($arg)*))
            .expect("Failed to write to stderr.")
    })
}

// Convert a string of %Y-%m-%d to a single u32 maintaining ordering.
fn str_to_ymd(ymd: &str) -> Option<u32> {
    let ymd: Vec<_> = ymd.split("-").filter_map(|s| s.parse::<u32>().ok()).collect();
    match ymd.as_slice() {
        &[y, m, d] => Some((y << 9) | (m << 5) | d),
        _ => None,
    }
}

fn main() {
    // Run rustc to get the version information.
    let output = env::var("RUSTC").ok()
        .and_then(|rustc| Command::new(rustc).arg("--version").output().ok())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|s| s.split(" ").nth(3).map(|s| s.to_string()))
        .map(|s| s.trim_right().trim_right_matches(")").to_string());

    if let Some(ref version) = output {
        let needed = str_to_ymd(MIN_VERSION);
        let actual = str_to_ymd(version);
        if let (Some(needed), Some(actual)) = (needed, actual) {
            if actual < needed {
                printerr!("{} {}",
                          Red.bold().paint("Error:"),
                          White.paint("Rocket codegen requires a newer version of rustc."));
                printerr!("{}{}{}",
                          Blue.paint("Use `"),
                          White.paint("rustup update"),
                          Blue.paint("` or your preferred method to update Rust."));
                printerr!("{} {}. {} {}.",
                          White.paint("Installed version is:"),
                          Yellow.paint(version.as_str()),
                          White.paint("Minimum required:"),
                          Yellow.paint(MIN_VERSION));
                panic!("Aborting compilation due to incompatible compiler.")
            } else {
                return;
            }
        }
    }

    printerr!("{}", Yellow.paint("Warning: Rocket was unable to check rustc compatibility."));
    printerr!("{}", Yellow.paint("Build may fail due to incompatible rustc version."));
}
