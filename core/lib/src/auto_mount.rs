pub struct AutoMountRoute {
    pub route: &'static ::StaticRouteInfo,
    pub mod_info: &'static AutoMountModuleInfo,
}

::inventory::collect!(AutoMountRoute);

pub struct AutoMountModuleInfo {
    pub base: &'static str,
    pub enabled: bool,
}

pub mod __default_auto_mount_info {
    use super::*;
    pub static __rocket_mod_auto_mount_info : AutoMountModuleInfo = AutoMountModuleInfo {base: "/", enabled: true};
}

/// Allows to configure behavior of [auto_mount()](Rocket::auto_mount) for all routes in module
///
/// # Example - set base path for all routes in module
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// use rocket::{mod_auto_mount,get};
///
/// mod secret_routes {
///     mod_auto_mount!("/foo");
///
///     // this route will be mounted at /foo/bar when auto_mount() is used
///     #[get("/bar")]
///     fn bar() -> &'static str {
///         "Hello!"
///     }
/// }
///
/// ```
///  # Example - disable automatic mounting for all routes in module
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// use rocket::{mod_auto_mount,get};
///
/// mod secret_routes {
///     mod_auto_mount!(disabled);
///
///     // this route will not be mounted by auto_mount()
///     #[get("/secret")]
///     fn secret() -> &'static str {
///         "secret route"
///     }
/// }
/// ```
#[macro_export]
macro_rules! mod_auto_mount {
    ($l:literal) => {
        use $crate::auto_mount::AutoMountModuleInfo;
        static __rocket_mod_auto_mount_info : AutoMountModuleInfo = AutoMountModuleInfo {base: $l, enabled: true};
    };
    (disabled) => {
        use $crate::auto_mount::AutoMountModuleInfo;
        static __rocket_mod_auto_mount_info : AutoMountModuleInfo = AutoMountModuleInfo {base: "/", enabled: false};
    }
}
