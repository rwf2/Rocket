//! Ensures Rocket isn't compiled with an incompatible version of Rust.

use yansi::{Paint, Color::{Red, Yellow}};

fn main() {
    const MIN_VERSION: &str = "1.46.0";

    if let Some(version) = version_check::Version::read() {
        if !version.at_least(MIN_VERSION) {
            let msg = "Rocket requires a more recent version of rustc.";
            eprintln!("{} {}", Red.paint("Error:").bold(), Paint::new(msg).bold());
            eprintln!("Installed version: {}", Yellow.paint(version));
            eprintln!("Minimum required:  {}", Yellow.paint(MIN_VERSION));
            panic!("Aborting compilation due to incompatible compiler.")
        }
    } else {
        println!("cargo:warning=Rocket was unable to check rustc compiler compatibility.");
        println!("cargo:warning=Build may fail due to incompatible rustc version.");
    }

    if let Some(true) = version_check::is_feature_flaggable() {
        println!("cargo:rustc-cfg=nightly");
    }
}
