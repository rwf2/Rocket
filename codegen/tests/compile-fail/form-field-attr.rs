#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

#[derive(FromForm)]
struct MyForm {
    #[form(field = "blah", field = "bloo")]
    //~^ ERROR: incorrect use of attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm1 {
    #[form]
    //~^ ERROR: incorrect use of attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm2 {
    #[form("blah")]
    //~^ ERROR: invalid `form` attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm3 {
    #[form(123)]
    //~^ ERROR: invalid `form` attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm4 {
    #[form(beep = "bop")]
    //~^ ERROR: invalid `form` attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm5 {
    #[form(field = "blah")]
    #[form(field = "blah")]
    my_field: String,
    //~^ ERROR: only a single
}

#[derive(FromForm)]
struct MyForm6 {
    #[form(field = true)]
    //~^ ERROR: invalid `field` in attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm7 {
    #[form(field)]
    //~^ ERROR: invalid `field` in attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm8 {
    #[form(field = 123)]
    //~^ ERROR: invalid `field` in attribute
    my_field: String,
}

#[derive(FromForm)]
struct MyForm9 {
    #[form(field = "hello")]
    first: String,
    #[form(field = "hello")]
    //~^ ERROR: field with duplicate name
    other: String,
}

#[derive(FromForm)]
struct MyForm10 {
    first: String,
    #[form(field = "first")]
    //~^ ERROR: field with duplicate name
    other: String,
}

#[derive(FromForm)]
struct MyForm11 {
    #[form(field = "hello world")]
    //~^ ERROR: invalid form field
    first: String,
}

#[derive(FromForm)]
struct MyForm12 {
    #[form(field = "!@#$%^&*()_")]
    //~^ ERROR: invalid form field
    first: String,
}
