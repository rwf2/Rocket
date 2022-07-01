fn main() {
    if let Some(true) = version_check::is_feature_flaggable() {
        println!("cargo:rustc-cfg=nightly");
    }
}
