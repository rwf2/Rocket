use crate::prelude::*;

fn test_ignite_failure() -> Result<()> {
    let server = spawn! {
        let fail = AdHoc::try_on_ignite("FailNow", |rocket| async { Err(rocket) });
        Rocket::default().attach(fail)
    };

    if let Err(Error::Liftoff(stdout, _)) = server {
        assert!(stdout.contains("failed ignite"));
        assert!(stdout.contains("FailNow"));
    } else {
        panic!("unexpected result: {server:#?}");
    }

    Ok(())
}

register!(test_ignite_failure);
