#![allow(dead_code, unused_variables)]

mod a {
    // async launch that is async.
    #[rocket::launch]
    async fn rocket() -> rocket::Rocket<rocket::Build> {
        let _ = rocket::build().launch().await;
        rocket::build()
    }

    async fn use_it() {
        let rocket: rocket::Rocket<rocket::Build> = rocket().await;
    }
}

mod b {
    // async launch that isn't async.
    #[rocket::launch]
    async fn main2() -> _ {
        rocket::build()
    }

    async fn use_it() {
        let rocket: rocket::Rocket<_> = main2().await;
    }
}

mod b_inferred {
    #[rocket::launch]
    async fn main2() -> _ { rocket::build() }

    async fn use_it() {
        let rocket: rocket::Rocket<_> = main2().await;
    }
}

mod c {
    // non-async launch.
    #[rocket::launch]
    fn rocket() -> _ {
        rocket::build()
    }

    fn use_it() {
        let rocket: rocket::Rocket<_> = rocket();
    }
}

mod c_inferred {
    #[rocket::launch]
    fn rocket() -> _ { rocket::build() }

    fn use_it() {
        let rocket: rocket::Rocket<_> = rocket();
    }
}

mod d {
    // main with async, is async.
    #[rocket::main]
    async fn main() {
        let _ = rocket::build().launch().await;
    }
}

mod e {
    // main with async, isn't async.
    #[rocket::main]
    async fn main() { }
}

mod f {
    // main with async, is async, with termination return.
    #[rocket::main]
    async fn main() -> Result<(), rocket::Error> {
        let _: rocket::Rocket<rocket::Ignite> = rocket::build().launch().await?;
        Ok(())
    }
}

mod g {
    // main with async, isn't async, with termination return.
    #[rocket::main]
    async fn main() -> Result<(), String> {
        Ok(())
    }
}

// main with async, is async, with termination return.
#[rocket::main]
async fn main() -> Result<(), String> {
    let result = rocket::build().launch().await;
    let _: rocket::Rocket<rocket::Ignite> = result.map_err(|e| e.to_string())?;
    Ok(())
}
