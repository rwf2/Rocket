use std::fmt;
use super::{rocket, FormInput, FormOption};

use rocket::local::Client;
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

async fn assert_form_eq(client: &Client, form_str: &str, expected: String) {
    let mut res = client.post("/")
        .header(ContentType::Form)
        .body(form_str)
        .dispatch().await;

    assert_eq!(res.body_string().await, Some(expected));
}

async fn assert_valid_form(client: &Client, input: &FormInput<'_>) {
    let f = format!("checkbox={}&number={}&type={}&password={}&textarea={}&select={}",
            input.checkbox, input.number, input.radio, input.password,
            input.text_area, input.select);
    assert_form_eq(client, &f, format!("{:?}", input)).await;
}

async fn assert_valid_raw_form(client: &Client, form_str: &str, input: &FormInput<'_>) {
    assert_form_eq(client, form_str, format!("{:?}", input)).await;
}

#[rocket::async_test]
async fn test_good_forms() {
    let client = Client::new(rocket()).await.unwrap();
    let mut input = FormInput {
        checkbox: true,
        number: 310,
        radio: FormOption::A,
        password: "beep".into(),
        text_area: "bop".to_string(),
        select: FormOption::B
    };

    assert_valid_form(&client, &input).await;

    input.checkbox = false;
    assert_valid_form(&client, &input).await;

    input.number = 0;
    assert_valid_form(&client, &input).await;
    input.number = 120;
    assert_valid_form(&client, &input).await;
    input.number = 133;
    assert_valid_form(&client, &input).await;

    input.radio = FormOption::B;
    assert_valid_form(&client, &input).await;
    input.radio = FormOption::C;
    assert_valid_form(&client, &input).await;

    input.password = "".into();
    assert_valid_form(&client, &input).await;
    input.password = "----90138490285u2o3hndslkv".into();
    assert_valid_form(&client, &input).await;
    input.password = "hi".into();
    assert_valid_form(&client, &input).await;

    input.text_area = "".to_string();
    assert_valid_form(&client, &input).await;
    input.text_area = "----90138490285u2o3hndslkv".to_string();
    assert_valid_form(&client, &input).await;
    input.text_area = "hey".to_string();
    assert_valid_form(&client, &input).await;

    input.select = FormOption::A;
    assert_valid_form(&client, &input).await;
    input.select = FormOption::C;
    assert_valid_form(&client, &input).await;

    // checkbox need not be present; defaults to false; accepts 'on' and 'off'
    assert_valid_raw_form(&client,
                          "number=133&type=c&password=hi&textarea=hey&select=c",
                          &input).await;

    assert_valid_raw_form(&client,
                          "checkbox=off&number=133&type=c&password=hi&textarea=hey&select=c",
                          &input).await;

    input.checkbox = true;
    assert_valid_raw_form(&client,
                          "checkbox=on&number=133&type=c&password=hi&textarea=hey&select=c",
                          &input).await;
}

async fn assert_invalid_form(client: &Client, vals: &mut [&str; 6]) {
    let s = format!("checkbox={}&number={}&type={}&password={}&textarea={}&select={}",
                    vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]);
    assert_form_eq(client, &s, format!("Invalid form input: {}", s)).await;
    *vals = ["true", "1", "a", "hi", "hey", "b"];
}

async fn assert_invalid_raw_form(client: &Client, form_str: &str) {
    assert_form_eq(client, form_str, format!("Invalid form input: {}", form_str)).await;
}

#[rocket::async_test]
async fn check_semantically_invalid_forms() {
    let client = Client::new(rocket()).await.unwrap();
    let mut form_vals = ["true", "1", "a", "hi", "hey", "b"];

    form_vals[0] = "not true";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[0] = "bing";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[0] = "true0";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[0] = " false";
    assert_invalid_form(&client, &mut form_vals).await;

    form_vals[1] = "-1";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[1] = "1e10";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[1] = "-1-1";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[1] = "NaN";
    assert_invalid_form(&client, &mut form_vals).await;

    form_vals[2] = "A?";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[2] = " B";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[2] = "d";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[2] = "100";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[2] = "";
    assert_invalid_form(&client, &mut form_vals).await;

    // password and textarea are always valid, so we skip them
    form_vals[5] = "A.";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[5] = "b ";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[5] = "d";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[5] = "-a";
    assert_invalid_form(&client, &mut form_vals).await;
    form_vals[5] = "";
    assert_invalid_form(&client, &mut form_vals).await;

    // now forms with missing fields
    assert_invalid_raw_form(&client, "number=10&type=a&password=hi&textarea=hey").await;
    assert_invalid_raw_form(&client, "number=10&radio=a&password=hi&textarea=hey&select=b").await;
    assert_invalid_raw_form(&client, "number=10&password=hi&select=b").await;
    assert_invalid_raw_form(&client, "number=10&select=b").await;
    assert_invalid_raw_form(&client, "password=hi&select=b").await;
    assert_invalid_raw_form(&client, "password=hi").await;
    assert_invalid_raw_form(&client, "").await;
}

#[rocket::async_test]
async fn check_structurally_invalid_forms() {
    let client = Client::new(rocket()).await.unwrap();
    assert_invalid_raw_form(&client, "==&&&&&&==").await;
    assert_invalid_raw_form(&client, "a&=b").await;
    assert_invalid_raw_form(&client, "=").await;
}

#[rocket::async_test]
async fn check_bad_utf8() {
    let client = Client::new(rocket()).await.unwrap();
    unsafe {
        let bad_str = std::str::from_utf8_unchecked(b"a=\xff");
        assert_form_eq(&client, bad_str, "Form input was invalid UTF-8.".into()).await;
    }
}
