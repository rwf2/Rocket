use std::fmt;
use super::{rocket, FormInput, FormOption};

use rocket::local::blocking::Client;
use rocket::http::ContentType;

impl fmt::Display for FormOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FormOption::A => write!(f, "a"),
            FormOption::B => write!(f, "b"),
            FormOption::C => write!(f, "c"),
        }
    }
}

macro_rules! assert_form_eq {
    ($client:expr, $form_str:expr, $expected:expr) => {{
        let res = $client.post("/")
            .header(ContentType::Form)
            .body($form_str)
            .dispatch();

        assert_eq!(res.into_string(), Some($expected));
    }};
}

macro_rules! assert_valid_form {
    ($client:expr, $input:expr) => {{
        let f = format!("checkbox={}&number={}&type={}&password={}&textarea={}&select={}",
                $input.checkbox, $input.number, $input.radio, $input.password,
                $input.text_area, $input.select);
        assert_form_eq!($client, &f, format!("{:?}", $input));
    }};
}

macro_rules! assert_valid_raw_form {
    ($client:expr, $form_str:expr, $input:expr) => {{
        assert_form_eq!($client, $form_str, format!("{:?}", $input));
    }};
}

#[test]
fn test_good_forms() {
    let client = Client::tracked(rocket()).unwrap();
    let mut input = FormInput {
        checkbox: true,
        number: 310,
        radio: FormOption::A,
        password: "beep".into(),
        text_area: "bop".to_string(),
        select: FormOption::B
    };

    assert_valid_form!(&client, &input);

    input.checkbox = false;
    assert_valid_form!(&client, &input);

    input.number = 0;
    assert_valid_form!(&client, &input);
    input.number = 120;
    assert_valid_form!(&client, &input);
    input.number = 133;
    assert_valid_form!(&client, &input);

    input.radio = FormOption::B;
    assert_valid_form!(&client, &input);
    input.radio = FormOption::C;
    assert_valid_form!(&client, &input);

    input.password = "".into();
    assert_valid_form!(&client, &input);
    input.password = "----90138490285u2o3hndslkv".into();
    assert_valid_form!(&client, &input);
    input.password = "hi".into();
    assert_valid_form!(&client, &input);

    input.text_area = "".to_string();
    assert_valid_form!(&client, &input);
    input.text_area = "----90138490285u2o3hndslkv".to_string();
    assert_valid_form!(&client, &input);
    input.text_area = "hey".to_string();
    assert_valid_form!(&client, &input);

    input.select = FormOption::A;
    assert_valid_form!(&client, &input);
    input.select = FormOption::C;
    assert_valid_form!(&client, &input);

    // checkbox need not be present; defaults to false; accepts 'on' and 'off'
    assert_valid_raw_form!(&client,
                          "number=133&type=c&password=hi&textarea=hey&select=c",
                          &input);

    assert_valid_raw_form!(&client,
                          "checkbox=off&number=133&type=c&password=hi&textarea=hey&select=c",
                          &input);

    input.checkbox = true;
    assert_valid_raw_form!(&client,
                          "checkbox=on&number=133&type=c&password=hi&textarea=hey&select=c",
                          &input);
}

macro_rules! assert_invalid_form {
    ($client:expr, $vals:expr) => {{
        let vals = $vals;
        let s = format!("checkbox={}&number={}&type={}&password={}&textarea={}&select={}",
                        vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]);
        assert_form_eq!($client, &s, format!("Invalid form input: {}", s));
        *vals = ["true", "1", "a", "hi", "hey", "b"];
    }};
}

macro_rules! assert_invalid_raw_form {
    ($client:expr, $form_str:expr) => {{
        assert_form_eq!($client, $form_str, format!("Invalid form input: {}", $form_str));
    }};
}

#[test]
fn check_semantically_invalid_forms() {
    let client = Client::tracked(rocket()).unwrap();
    let mut form_vals = ["true", "1", "a", "hi", "hey", "b"];

    form_vals[0] = "not true";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[0] = "bing";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[0] = "true0";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[0] = " false";
    assert_invalid_form!(&client, &mut form_vals);

    form_vals[1] = "-1";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[1] = "1e10";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[1] = "-1-1";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[1] = "NaN";
    assert_invalid_form!(&client, &mut form_vals);

    form_vals[2] = "A?";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[2] = " B";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[2] = "d";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[2] = "100";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[2] = "";
    assert_invalid_form!(&client, &mut form_vals);

    // password and textarea are always valid, so we skip them
    form_vals[5] = "A.";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[5] = "b ";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[5] = "d";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[5] = "-a";
    assert_invalid_form!(&client, &mut form_vals);
    form_vals[5] = "";
    assert_invalid_form!(&client, &mut form_vals);

    // now forms with missing fields
    assert_invalid_raw_form!(&client, "number=10&type=a&password=hi&textarea=hey");
    assert_invalid_raw_form!(&client, "number=10&radio=a&password=hi&textarea=hey&select=b");
    assert_invalid_raw_form!(&client, "number=10&password=hi&select=b");
    assert_invalid_raw_form!(&client, "number=10&select=b");
    assert_invalid_raw_form!(&client, "password=hi&select=b");
    assert_invalid_raw_form!(&client, "password=hi");
    assert_invalid_raw_form!(&client, "");
}

#[test]
fn check_structurally_invalid_forms() {
    let client = Client::tracked(rocket()).unwrap();
    assert_invalid_raw_form!(&client, "==&&&&&&==");
    assert_invalid_raw_form!(&client, "a&=b");
    assert_invalid_raw_form!(&client, "=");
}

#[test]
fn check_bad_utf8() {
    let client = Client::tracked(rocket()).unwrap();
    unsafe {
        let bad_str = std::str::from_utf8_unchecked(b"a=\xff");
        assert_form_eq!(&client, bad_str, "Form input was invalid UTF-8.".into());
    }
}
