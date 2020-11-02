#![allow(dead_code, unused_variables)]

mod a {
    // async launch that is async.
    #[rocket::launch]
    async fn rocket() -> rocket::Rocket {
        let _ = rocket::ignite().launch().await;
        rocket::ignite()
    }

    async fn use_it() {
        let rocket: rocket::Rocket = rocket().await;
    }
}

mod b {
    // async launch that isn't async.
    #[rocket::launch]
    async fn main2() -> rocket::Rocket {
        rocket::ignite()
    }

    async fn use_it() {
        let rocket: rocket::Rocket = main2().await;
    }
}

mod b_inferred {
    #[rocket::launch]
    async fn main2() -> _ { rocket::ignite() }

    async fn use_it() {
        let rocket: rocket::Rocket = main2().await;
    }
}

mod c {
    // non-async launch.
    #[rocket::launch]
    fn rocket() -> rocket::Rocket {
        rocket::ignite()
    }

    fn use_it() {
        let rocket: rocket::Rocket = rocket();
    }
}

mod c_inferred {
    #[rocket::launch]
    fn rocket() -> _ { rocket::ignite() }

    fn use_it() {
        let rocket: rocket::Rocket = rocket();
    }
}

mod d {
    // main with async, is async.
    #[rocket::main]
    async fn main() {
        let _ = rocket::ignite().launch().await;
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
    async fn main() -> Result<(), String> {
        let result = rocket::ignite().launch().await;
        result.map_err(|e| e.to_string())
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
    let result = rocket::ignite().launch().await;
    result.map_err(|e| e.to_string())
}
