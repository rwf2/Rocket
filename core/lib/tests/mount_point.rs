#[test]
#[should_panic]
fn bad_dynamic_mount() {
    let _ = rocket::build().mount("<name>", vec![]);
}

#[test]
#[should_panic]
fn bad_dynamic_utf8_mount() {
    let _ = rocket::build().mount("/häcker/<x>", vec![]);
}

#[test]
#[should_panic]
fn bad_dynamic_nested_mount() {
    let _ = rocket::build().mount("/a/<x>", vec![]);
}

#[test]
#[should_panic]
fn bad_dynamic_register() {
    let _ = rocket::build().register("/a/<x>", vec![]);
}

#[test]
fn good_static_mount() {
    let _ = rocket::build().mount("/abcdefghijkl_mno", vec![]);
}

#[test]
fn good_utf8_mount() {
    let _ = rocket::build().mount("/häcker", vec![]).register("/häcker", vec![]);
}

#[test]
fn nested_utf8_catcher_rebase() {
    use rocket::Catcher;
    use rocket::catcher::dummy_handler;

    let catcher = Catcher::new(404, dummy_handler).rebase("/wörld").rebase("/häcker");
    assert_eq!(catcher.base(), "/häcker/wörld");
}

#[test]
fn whitespace_bases() {
    use rocket::{Route, Catcher};
    use rocket::http::Method;

    let route = || Route::new(Method::Get, "/", rocket::route::dummy_handler);
    let catcher = || Catcher::new(404, rocket::catcher::dummy_handler);
    for base in ["/a ", "/a/ ", "/ "] {
        let rebased = route().rebase(base);
        assert_eq!(rebased.uri.base(), base);
        assert_eq!(rebased.uri.path(), base);
        assert_eq!(route().map_base(|_| base.into()).unwrap().uri, base);
        assert_eq!(catcher().rebase(base).base(), base);
        assert_eq!(catcher().map_base(|_| base.into()).unwrap().base(), base);
    }

    assert_eq!(route().rebase("/b ").rebase("/a ").uri, "/a /b ");
    assert_eq!(catcher().rebase("/b ").rebase("/a ").base(), "/a /b ");

    assert!(!route().rebase("/a ").collides_with(&route().rebase("/a")));
    assert!(!catcher().rebase("/a ").collides_with(&catcher().rebase("/a")));
}
