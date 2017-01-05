extern crate compiletest_rs as compiletest;

use std::path::PathBuf;

fn run_mode(mode: &'static str) {
    let mut config = compiletest::default_config();
    let cfg_mode = mode.parse().expect("Invalid mode");

    config.mode = cfg_mode;
    config.src_base = PathBuf::from(format!("tests/{}", mode));
    let flags = [
        "-L ../target/debug/",
        "-L ../target/debug/deps/",
        "-L target/debug/",
        "-L target/debug/deps/"
    ].join(" ");

    config.target_rustcflags = Some(flags);
    compiletest::run_tests(&config);
}

#[test]
fn compile_test() {
    run_mode("compile-fail");
    run_mode("run-pass");
}
