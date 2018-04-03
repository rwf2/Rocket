pub mod recipes {
    use rocket_contrib::Template;
    use rocket::response::Redirect;
    use rocket::request::Form;
    use schema::recipes::dsl::*;
    use diesel::prelude::*;
    use diesel::LimitDsl;
    use diesel::LoadDsl;

    use models;
    use forms;
    use database;

    #[get("/")]
    pub fn index() -> Template {
        let connection = database::establish_connection();
        let results = recipes
            .limit(20)
            .load::<models::Recipe>(&connection)
            .expect("Error loading recipes");

        Template::render("recipes/index", &results)
    }

    #[get("/<recipe_id>")]
    pub fn show(recipe_id: i32) -> Template {
        let connection = database::establish_connection();
        let results = recipes
            .filter(id.eq(recipe_id))
            .limit(1)
            .load::<models::Recipe>(&connection)
            .expect("Error loading recipes");
        Template::render("recipes/show", results.first())
    }


    #[get("/new")]
    pub fn new() -> Template {
        Template::render("recipes/new", &())
    }

    #[post("/", data = "<form_data>")]
    pub fn create(form_data: Form<forms::Recipe>) -> Redirect {
        use diesel;

        let connection = database::establish_connection();
        let recipe = models::NewRecipe {
            id: None,
            name: form_data.get().name.to_string(),
        };

        match diesel::insert(&recipe).into(recipes).execute(&connection) {
            Ok(_) => Redirect::to("/recipes"),
            Err(error) => panic!("There was a problem opening the file: {:?}", error),
        }
    }

    #[get("/<recipe_id>/edit")]
    pub fn edit(recipe_id: i32) -> Template {
        let connection = database::establish_connection();
        let results = recipes
            .filter(id.eq(recipe_id))
            .limit(1)
            .load::<models::Recipe>(&connection)
            .expect("Error loading recipes");
        Template::render("recipes/edit", results.first())
    }

    #[put("/<recipe_id>", data = "<form_data>")]
    pub fn update(recipe_id: i32, form_data: Form<forms::Recipe>) -> Redirect {
        use diesel;

        let connection = database::establish_connection();

        let result = diesel::update(recipes.find(recipe_id))
            .set(name.eq(form_data.get().name.to_string()))
            .execute(&connection);


        match result {
            Ok(_) => Redirect::to(&format!("/recipes/{}", recipe_id)),
            Err(error) => panic!("There was a problem opening the file: {:?}", error),
        }
    }

    #[delete("/<recipe_id>")]
    pub fn delete(recipe_id: i32) -> Redirect {
        use diesel;

        let connection = database::establish_connection();

        match diesel::delete(recipes.find(recipe_id)).execute(&connection) {
            Ok(_) => Redirect::to("/recipes"),
            Err(error) => panic!("There was a problem opening the file: {:?}", error),
        }
    }
}

pub mod pages {
    use rocket_contrib::Template;

    #[get("/")]
    pub fn home() -> Template {
        let map = ();
        Template::render("pages/home", &map)
    }
}
