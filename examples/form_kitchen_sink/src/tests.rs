use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::ContentType;

use super::rocket;
use super::FormInput;
use super::FormOption;

use std::fmt;

impl fmt::Display for FormOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let formatted_input = match *self {
            FormOption::A => "a",
            FormOption::B => "b",
            FormOption::C => "c",
        };

        write!(f, "{}", formatted_input)
    }
}

fn test_post_form(request_form: &FormInput) -> String {
    let rocket = rocket();
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
    response.body_string().unwrap()
}

#[test]
fn test_good_form() {
    let form_input = FormInput {
        checkbox: true,
        number: 1,
        radio: FormOption::A,
        password: "password".to_string(),
        text_area: "text_area".to_string(),
        select: FormOption::B,
    };

    let response = test_post_form(&form_input);

    assert_eq!(response, format!("{:?}", form_input));
}