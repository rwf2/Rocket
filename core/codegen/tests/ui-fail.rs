#[test]
fn ui() {
    let path = match version_check::is_feature_flaggable() {
        Some(true) => "ui-fail-nightly",
        _ => "ui-fail-stable"
    };

    let glob = std::env::args().last()
        .map(|arg| format!("*{}*.rs", arg))
        .unwrap_or_else(|| "*.rs".into());

    let t = trybuild::TestCases::new();
    t.compile_fail(format!("tests/{}/{}", path, glob));
}
