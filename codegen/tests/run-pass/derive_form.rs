#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::request::{FromForm, FromFormValue};

#[derive(Debug, PartialEq, FromForm)]
struct TodoTask {
    description: String,
    completed: bool
}

// TODO: Make deriving `FromForm` for this enum possible.
#[derive(Debug, PartialEq)]
enum FormOption {
    A, B, C
}

impl<'v> FromFormValue<'v> for FormOption {
    type Error = &'v str;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        let variant = match v {
            "a" => FormOption::A,
            "b" => FormOption::B,
            "c" => FormOption::C,
            _ => return Err(v)
        };

        Ok(variant)
    }
}

#[derive(Debug, PartialEq, FromForm)]
struct FormInput<'r> {
    checkbox: bool,
    number: usize,
    radio: FormOption,
    password: &'r str,
    textarea: String,
    select: FormOption,
}

#[derive(Debug, PartialEq, FromForm)]
struct DefaultInput<'r> {
    arg: Option<&'r str>,
}

#[derive(Debug, PartialEq, FromForm)]
struct ManualMethod<'r> {
    _method: Option<&'r str>,
    done: bool
}

fn main() {
    // Same number of arguments: simple case.
    let task = TodoTask::from_form_string("description=Hello&completed=on");
    assert_eq!(task, Ok(TodoTask {
        description: "Hello".to_string(),
        completed: true
    }));

    // Argument in string but not in form.
    let task = TodoTask::from_form_string("other=a&description=Hello&completed=on");
    assert!(task.is_err());

    // Ensure _method isn't required.
    let task = TodoTask::from_form_string("_method=patch&description=Hello&completed=off");
    assert_eq!(task, Ok(TodoTask {
        description: "Hello".to_string(),
        completed: false
    }));

    let form_string = &[
        "password=testing", "checkbox=off", "checkbox=on", "number=10",
        "checkbox=off", "textarea=", "select=a", "radio=c",
    ].join("&");

    let input = FormInput::from_form_string(&form_string);
    assert_eq!(input, Ok(FormInput {
        checkbox: false,
        number: 10,
        radio: FormOption::C,
        password: "testing",
        textarea: "".to_string(),
        select: FormOption::A,
    }));

    // Argument not in string with default in form.
    let default = DefaultInput::from_form_string("");
    assert_eq!(default, Ok(DefaultInput {
        arg: None
    }));

    // Ensure _method can be captured if desired.
    let manual = ManualMethod::from_form_string("_method=put&done=true");
    assert_eq!(manual, Ok(ManualMethod {
        _method: Some("put"),
        done: true
    }));

    // And ignored when not present.
    let manual = ManualMethod::from_form_string("done=true");
    assert_eq!(manual, Ok(ManualMethod {
        _method: None,
        done: true
    }));
}
