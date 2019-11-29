#![feature(prelude_import)]
#![feature(proc_macro_hygiene)]
#![feature(const_type_id)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;

#[macro_use]
extern crate rocket;

use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::response::content;
use rocket::State;

struct HitCount(AtomicUsize);

struct HitCountUnused(AtomicUsize);

struct HitCountUnmanaged(AtomicUsize);

fn index(hit_count: State<'_, HitCount>, hhh: State<HitCountUnmanaged>) -> content::Html<String> {
    hhh.0.fetch_add(1, Ordering::Relaxed);
    hit_count.0.fetch_add(1, Ordering::Relaxed);
    let msg = "Your visit has been recorded!";
    let count = {
        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
            &["Visits: "],
            &match (&count(hit_count),) {
                (arg0,) => [::core::fmt::ArgumentV1::new(
                    arg0,
                    ::core::fmt::Display::fmt,
                )],
            },
        ));
        res
    };
    content::Html({
        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
            &["", "<br /><br />"],
            &match (&msg, &count) {
                (arg0, arg1) => [
                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                ],
            },
        ));
        res
    })
}
#[doc = r" Rocket code generated wrapping route function."]
fn rocket_route_fn_index<'_b>(
    __req: &'_b rocket::Request,
    __data: rocket::Data,
) -> rocket::handler::Outcome<'_b> {
    #[allow(non_snake_case, unreachable_patterns, unreachable_code)]
    let __rocket_param_hit_count: State<'_, HitCount> =
        match <State<'_, HitCount> as rocket::request::FromRequest>::from_request(__req) {
            rocket::Outcome::Success(__v) => __v,
            rocket::Outcome::Forward(_) => return rocket::Outcome::Forward(__data),
            rocket::Outcome::Failure((__c, _)) => return rocket::Outcome::Failure(__c),
        };
    #[allow(non_snake_case, unreachable_patterns, unreachable_code)]
    let __rocket_param_hhh: State<HitCountUnmanaged> =
        match <State<HitCountUnmanaged> as rocket::request::FromRequest>::from_request(__req) {
            rocket::Outcome::Success(__v) => __v,
            rocket::Outcome::Forward(_) => return rocket::Outcome::Forward(__data),
            rocket::Outcome::Failure((__c, _)) => return rocket::Outcome::Failure(__c),
        };
    let ___responder = index(__rocket_param_hit_count, __rocket_param_hhh);
    rocket::handler::Outcome::from(__req, ___responder)
}
#[doc = r" Rocket code generated wrapping URI macro."]
#[doc(hidden)]
#[macro_export]
macro_rules! rocket_uri_macro_index5078984740353793844 {
    ($ ($ token : tt) *) =>
    {
        {
            extern crate std ; extern crate rocket ; rocket ::
            rocket_internal_uri ! ("/", (), $ ($ token) *)
        }
    } ;
}
#[doc(hidden)]
pub use rocket_uri_macro_index5078984740353793844 as rocket_uri_macro_index;
#[doc = r" Rocket code generated static route info."]
#[allow(non_upper_case_globals)]
static static_rocket_route_info_for_index: rocket::StaticRouteInfo = rocket::StaticRouteInfo {
    name: "index",
    method: ::rocket::http::Method::Get,
    path: "/",
    handler: rocket_route_fn_index,
    format: None,
    rank: None,
    states: [
        Some(std::any::TypeId::of::<State<'_, HitCount>>()),
        Some(std::any::TypeId::of::<State<HitCountUnmanaged>>()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ],
};
fn count(hit_count: State<'_, HitCount>) -> String {
    hit_count.0.load(Ordering::Relaxed).to_string()
}
#[doc = r" Rocket code generated wrapping route function."]
fn rocket_route_fn_count<'_b>(
    __req: &'_b rocket::Request,
    __data: rocket::Data,
) -> rocket::handler::Outcome<'_b> {
    #[allow(non_snake_case, unreachable_patterns, unreachable_code)]
    let __rocket_param_hit_count: State<'_, HitCount> =
        match <State<'_, HitCount> as rocket::request::FromRequest>::from_request(__req) {
            rocket::Outcome::Success(__v) => __v,
            rocket::Outcome::Forward(_) => return rocket::Outcome::Forward(__data),
            rocket::Outcome::Failure((__c, _)) => return rocket::Outcome::Failure(__c),
        };
    let ___responder = count(__rocket_param_hit_count);
    rocket::handler::Outcome::from(__req, ___responder)
}
#[doc = r" Rocket code generated wrapping URI macro."]
#[doc(hidden)]
#[macro_export]
macro_rules! rocket_uri_macro_count936835158112775348 {
    ($ ($ token : tt) *) =>
    {
        {
            extern crate std ; extern crate rocket ; rocket ::
            rocket_internal_uri ! ("/count", (), $ ($ token) *)
        }
    } ;
}
#[doc(hidden)]
pub use rocket_uri_macro_count936835158112775348 as rocket_uri_macro_count;
#[doc = r" Rocket code generated static route info."]
#[allow(non_upper_case_globals)]
static static_rocket_route_info_for_count: rocket::StaticRouteInfo = rocket::StaticRouteInfo {
    name: "count",
    method: ::rocket::http::Method::Get,
    path: "/count",
    handler: rocket_route_fn_count,
    format: None,
    rank: None,
    states: [
        Some(std::any::TypeId::of::<State<'_, HitCount>>()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ],
};
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", {
            let __vector: Vec<::rocket::Route> = <[_]>::into_vec(box [
                ::rocket::Route::from(&static_rocket_route_info_for_index),
                ::rocket::Route::from(&static_rocket_route_info_for_count),
            ]);
            __vector
        })
        .manage(HitCount(AtomicUsize::new(0)))
        .manage(HitCountUnused(AtomicUsize::new(0)))
}
fn main() {
    rocket().launch();
}
