#![allow(dead_code)]

// rocket::main

mod main_a {
    #[rocket::main]
    fn foo() { }
    //~^^ ERROR `async`
}

mod main_b {
    #[rocket::main]
    async fn foo() { }
    //~^^ WARNING `main`
}

mod main_d {
    #[rocket::main]
    fn main() {
        //~^^ ERROR `async`
        let _ = rocket::ignite().launch().await;
    }
}

mod main_f {
    #[rocket::main]
    async fn main() {
        //~^ ERROR mismatched types
        rocket::ignite()
    }
}

// rocket::launch

mod launch_a {
    #[rocket::launch]
    async fn rocket() -> String {
        //~^ ERROR mismatched types
        let _ = rocket::ignite().launch().await;
        rocket::ignite()
        //~^ ERROR mismatched types
    }
}

mod launch_b {
    #[rocket::launch]
    async fn rocket() -> rocket::Rocket {
        let _ = rocket::ignite().launch().await;
        "hi".to_string()
        //~^ ERROR mismatched types
    }
}

mod launch_c {
    #[rocket::launch]
    fn main() -> rocket::Rocket {
        //~^^ ERROR `main`
        rocket::ignite()
    }
}

mod launch_d {
    #[rocket::launch]
    async fn rocket() {
        //~^^ ERROR functions that return
        let _ = rocket::ignite().launch().await;
        rocket::ignite()
    }
}

mod launch_e {
    #[rocket::launch]
    fn rocket() {
        //~^^ ERROR functions that return
        rocket::ignite()
    }
}

mod launch_f {
    #[rocket::launch]
    fn rocket() -> rocket::Rocket {
        let _ = rocket::ignite().launch().await;
        //~^ ERROR only allowed inside `async`
        rocket::ignite()
    }
}

mod launch_g {
    #[rocket::launch]
    fn main() -> &'static str {
        //~^^ ERROR `main`
        let _ = rocket::ignite().launch().await;
        "hi"
    }
}

mod launch_h {
    #[rocket::launch]
    async fn main() -> rocket::Rocket {
        //~^^ ERROR `main`
        rocket::ignite()
    }
}

#[rocket::main]
async fn main() -> rocket::Rocket {
    //~^ ERROR invalid return type
    rocket::ignite()
}
