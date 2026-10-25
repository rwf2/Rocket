fn main() {
    println!("cargo:rustc-check-cfg=cfg(nightly)");
    if let Some(true) = version_check::is_feature_flaggable() {
        println!("cargo:rustc-cfg=nightly");
    }
}
