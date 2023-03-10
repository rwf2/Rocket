//! Upgrade wrapper to deal with hyper::upgarde::Upgraded

use crate::http::hyper;

/// Trait to determine if any given response in rocket is upgradeable.
/// 
/// When a response has the http code 101 SwitchingProtocols, and the response implements the Upgrade trait,
/// then rocket aquires the hyper::upgarde::Upgraded struct and calls the start() method of the trait with the hyper upgrade
/// and awaits the result.
#[crate::async_trait]
pub trait Upgrade<'a> {
    /// Called with the hyper::upgarde::Upgraded struct when a rocket response should be upgraded
    async fn start(&self, upgraded: hyper::upgrade::Upgraded);
}
