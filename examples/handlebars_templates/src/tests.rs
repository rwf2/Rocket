use rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::Status;
use rocket::Response;

macro_rules! run_test {
    ($req:expr, $test_fn:expr) => ({
        let rocket = rocket::ignite()
            .mount("/", routes![super::index, super::get])
            .catch(errors![super::not_found]);

        $test_fn($req.dispatch_with(&rocket));
    })
}

#[test]
fn test_root() {
    // Check that the redirect works.
    for method in [Get, Head].iter() {
        let mut req = MockRequest::new(*method, "/");
        run_test!(req, |mut response: Response| {
            assert_eq!(response.status(), Status::SeeOther);
            
            assert!(response.body().is_none());
            
            let location_headers: Vec<_> = response.header_values("Location").collect();
            let content_length: Vec<_> = response.header_values("Content-Length").collect();
            
            assert_eq!(location_headers, vec!["/hello/Unknown"]);
            assert_eq!(content_length, vec!["0"]);
        });
    }
    
    // Check that other request methods are not accepted (and instead caught).
    for method in [Post, Put, Delete, Options, Trace, Connect, Patch].iter() {
        let mut req = MockRequest::new(*method, "/");
        run_test!(req, |mut response: Response| {
            assert_eq!(response.status(), Status::NotFound);
            
            let body_string = response.body().and_then(|body| body.into_string());
            assert_eq!(body_string, Some("<!DOCTYPE html>\n<html>\n  <head>\n    <meta charset=\"utf-8\" />\n    <title>404</title>\n  </head>\n  <body>\n    <h1>404: Hey! There\'s nothing here.</h1>\n    The page at / does not exist!\n  </body>\n</html>\n".to_string()));
        });
    }
}

#[test]
fn test_name() {
    // Check that the /hello/<name> route works.
    let mut req = MockRequest::new(Get, "/hello/Jack");
    run_test!(req, |mut response: Response| {
        assert_eq!(response.status(), Status::Ok);
        
        let body_string = response.body().and_then(|body| body.into_string());
        assert_eq!(body_string, Some("<!DOCTYPE html>\n<html>\n  <head>\n    <meta charset=\"utf-8\" />\n    <title>Handlebars Demo</title>\n  </head>\n  <body>\n    <h1>Hi Jack</h1>\n    <h3>Here are your items:</h3>\n    <ul>\n      \n        <li>One</li>\n      \n        <li>Two</li>\n      \n        <li>Three</li>\n      \n    </ul>\n\n    <p>Try going to <a href=\"/hello/YourName\">/hello/YourName</a></p>\n  </body>\n</html>\n".to_string()));
    });
}

#[test]
fn test_404() {
    // Check that the error catcher works.
    let mut req = MockRequest::new(Get, "/hello/");
    run_test!(req, |mut response: Response| {
        assert_eq!(response.status(), Status::NotFound);
        
        let body_string = response.body().and_then(|body| body.into_string());
        assert_eq!(body_string, Some("<!DOCTYPE html>\n<html>\n  <head>\n    <meta charset=\"utf-8\" />\n    <title>404</title>\n  </head>\n  <body>\n    <h1>404: Hey! There\'s nothing here.</h1>\n    The page at /hello/ does not exist!\n  </body>\n</html>\n".to_string()));
    });
}
