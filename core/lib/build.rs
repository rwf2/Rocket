fn main() {
    if let Some((_, channel, _)) = version_check::triple() {
        if channel.supports_features() {
            println!("cargo:rustc-cfg=nightly");
        }
    }
}
