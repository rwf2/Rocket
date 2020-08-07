#[cfg(any(test, doctest))]
mod site_guide {
    rocket::rocket_internal_guide_tests!("../guide/*.md");
}

#[cfg(any(test, doctest))]
mod readme {
    doc_comment::doctest!("../../../README.md", readme);
}
