#[test]
#[ignore]
fn ui() {
    let path = match version_check::is_feature_flaggable() {
        Some(true) => "ui-fail-nightly",
        _ => "ui-fail-stable"
    };

    let t = trybuild::TestCases::new();
    t.compile_fail(format!("tests/{}/*.rs", path));
}
