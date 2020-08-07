#![allow(dead_code)]

// rocket::main

mod main_a {
    #[rocket::main]
    fn foo() { }

}

mod main_b {
    #[rocket::main]
    async fn foo() { }

}

mod main_d {
    #[rocket::main]
    fn main() {
        let _ = rocket::ignite().launch().await;
    }
}

mod main_f {
    #[rocket::main]
    async fn main() {
        rocket::ignite()
    }
}

// launch

mod launch_a {
    #[rocket::launch]
    async fn rocket() -> String {
        let _ = rocket::ignite().launch().await;
        rocket::ignite()

    }
}

mod launch_b {
    #[rocket::launch]
    async fn rocket() -> rocket::Rocket {
        let _ = rocket::ignite().launch().await;
        "hi".to_string()
    }
}

mod launch_c {
    #[rocket::launch]
    fn main() -> rocket::Rocket {
        rocket::ignite()
    }
}

mod launch_d {
    #[rocket::launch]
    async fn rocket() {
        let _ = rocket::ignite().launch().await;
        rocket::ignite()
    }
}

mod launch_e {
    #[rocket::launch]
    fn rocket() {
        rocket::ignite()
    }
}

mod launch_f {
    #[rocket::launch]
    fn rocket() -> rocket::Rocket {
        let _ = rocket::ignite().launch().await;
        rocket::ignite()
    }
}

mod launch_g {
    #[rocket::launch]
    fn main() -> &'static str {
        let _ = rocket::ignite().launch().await;
        "hi"
    }
}

mod launch_h {
    #[rocket::launch]
    async fn main() -> rocket::Rocket {
        rocket::ignite()
    }
}

#[rocket::main]
async fn main() -> rocket::Rocket {
    rocket::ignite()
}
