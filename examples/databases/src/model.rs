pub struct Person {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub username: String,
}
