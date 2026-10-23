fn main() {
    println!("cargo:rustc-check-cfg=cfg(nightly)");
    println!("cargo:rustc-check-cfg=cfg(broken_fmt)");

    if let Some((version, channel, _)) = version_check::triple() {
        if channel.supports_features() {
            println!("cargo:rustc-cfg=nightly");
        }

        if version.at_least("1.67") && version.at_most("1.68.2") {
            println!("cargo:rustc-cfg=broken_fmt");
        }
    }
}
