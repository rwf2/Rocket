use {Rocket, Request};
use local::LocalRequest;
use http::Method;
use http::uri::URI;
use error::LaunchError;

pub struct Client {
    rocket: Rocket,
}

impl Client {
    #[inline]
    pub fn new(rocket: Rocket) -> Result<Client, LaunchError> {
        if let Some(err) = rocket.prelaunch_check() {
            return Err(err);
        }

        Ok(Client {
            rocket: rocket,
        })
    }

    #[inline(always)]
    pub fn rocket(&self) -> &Rocket {
        &self.rocket
    }

    #[inline(always)]
    pub fn req<'c, 'u: 'c, U>(&'c self, method: Method, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        let request = Request::new(&self.rocket, method, uri);
        LocalRequest::new(&self.rocket, request)
    }

    #[inline(always)]
    pub fn get<'c, 'u: 'c, U: Into<URI<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Get, uri)
    }

    #[inline(always)]
    pub fn put<'c, 'u: 'c, U: Into<URI<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Put, uri)
    }

    #[inline(always)]
    pub fn post<'c, 'u: 'c, U: Into<URI<'u>>>(&'c self, uri: U) -> LocalRequest<'c> {
        self.req(Method::Post, uri)
    }

    #[inline(always)]
    pub fn delete<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Delete, uri)
    }

    #[inline(always)]
    pub fn options<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Options, uri)
    }

    #[inline(always)]
    pub fn head<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Head, uri)
    }

    #[inline(always)]
    pub fn patch<'c, 'u: 'c, U>(&'c self, uri: U) -> LocalRequest<'c>
        where U: Into<URI<'u>>
    {
        self.req(Method::Patch, uri)
    }
}
