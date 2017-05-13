use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::ContentType;

use super::get_rocket;
use super::FormInput;
use super::FormOption;

use std::fmt;

impl fmt::Display for FormOption {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let formatted_input = match *self {
            FormOption::A => "a",
            FormOption::B => "b",
            FormOption::C => "c",
        };
        write!(formatter, "{}", formatted_input)
    }
}

fn test_post_form(request_form: FormInput) -> String {
    let rocket = get_rocket();
    let mut request = MockRequest::new(Post, "/")
        .header(ContentType::Form)
        .body(&format!("checkbox={}&number={}&password={}&type={}&textarea={}&select={}",
            request_form.checkbox,
            request_form.number,
            request_form.password,
            request_form.radio,
            request_form.text_area,
            request_form.select));

    let mut response = request.dispatch_with(&rocket);
    let body_string = response.body_string().unwrap();

    body_string
}

#[test]
fn test_good_form() {
    let request = FormInput {
        checkbox: true,
        number: 1,
        radio: FormOption::A,
        password: "password".to_string(),
        text_area: "text_area".to_string(),
        select: FormOption::B,
    };

    let response = test_post_form(request);

    assert_eq!(response,
               r#"FormInput { checkbox: true, number: 1, radio: A, password: "password", text_area: "text_area", select: B }"#);
}