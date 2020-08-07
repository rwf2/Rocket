#![allow(dead_code)] // This test is only here to ensure it compiles.
#![allow(unused_variables)] // This test is only here to ensure it compiles.

mod a {
    /// Docs.
    #[rocket::post("/typed_uris/<id>")]
    fn simple(id: i32) { }
}

mod b {
    /// Docs.
    #[rocket::post("/typed_uris/<id>")]
    fn simple(id: i32) { }
}
