use std::fmt;
use std::rc::Rc;
use std::mem::transmute;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

use {Rocket, Request, Response, Data};
use http::{Header, Cookie};

pub struct LocalRequest<'c> {
    rocket: &'c Rocket,
    ptr: *mut Request<'c>,
    request: Rc<Request<'c>>,
    data: Vec<u8>
}

pub struct LocalResponse<'c> {
    _request: Rc<Request<'c>>,
    response: Response<'c>,
}

impl<'c> Deref for LocalResponse<'c> {
    type Target = Response<'c>;

    #[inline(always)]
    fn deref(&self) -> &Response<'c> {
        &self.response
    }
}

impl<'c> DerefMut for LocalResponse<'c> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Response<'c> {
        &mut self.response
    }
}

impl<'c> LocalRequest<'c> {
    #[inline(always)]
    pub fn new(rocket: &'c Rocket, request: Request<'c>) -> LocalRequest<'c> {
        let mut req = Rc::new(request);
        let ptr = Rc::get_mut(&mut req).unwrap() as *mut Request;
        LocalRequest { rocket: rocket, ptr: ptr, request: req, data: vec![] }
    }

    #[inline]
    pub fn inner(&self) -> &Request<'c> {
        &*self.request
    }

    #[inline(always)]
    fn request(&mut self) -> &mut Request<'c> {
        unsafe { &mut *self.ptr }
    }

    #[inline(always)]
    pub fn dispatch(mut self) -> LocalResponse<'c> {
        let req = unsafe { transmute(self.request()) };
        let response = self.rocket.dispatch(req, Data::local(self.data));

        LocalResponse {
            _request: self.request,
            response: response
        }
    }

    #[inline(always)]
    pub fn mut_dispatch(&mut self) -> LocalResponse<'c> {
        let data = ::std::mem::replace(&mut self.data, vec![]);
        let req = unsafe { transmute(self.request()) };
        let response = self.rocket.dispatch(req, Data::local(data));

        LocalResponse {
            _request: self.request.clone(),
            response: response
        }
    }

    #[inline(always)]
    pub fn cloned_dispatch(&self) -> LocalResponse<'c> {
        let cloned = (*self.request).clone();
        let mut req = LocalRequest::new(self.rocket, cloned);
        req.data = self.data.clone();
        req.dispatch()
    }

    /// Add a header to this request.
    ///
    /// # Examples
    ///
    /// Add the Content-Type header:
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::ContentType;
    ///
    /// # #[allow(unused_variables)]
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let req = client.get("/").header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn header<H: Into<Header<'static>>>(mut self, header: H) -> Self {
        self.request().add_header(header.into());
        self
    }

    /// Adds a header to this request without consuming `self`.
    ///
    /// # Examples
    ///
    /// Add the Content-Type header:
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let mut req = client.get("/");
    /// req.add_header(ContentType::JSON);
    /// ```
    #[inline]
    pub fn add_header<H: Into<Header<'static>>>(&mut self, header: H) {
        self.request().add_header(header.into());
    }

    /// Set the remote address of this request.
    ///
    /// # Examples
    ///
    /// Set the remote address to "8.8.8.8:80":
    ///
    /// ```rust
    /// use rocket::local::Client;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// let address = "8.8.8.8:80".parse().unwrap();
    /// let req = client.get("/").remote(address);
    /// ```
    #[inline]
    pub fn remote(mut self, address: SocketAddr) -> Self {
        self.request().set_remote(address);
        self
    }

    /// Add a cookie to this request.
    ///
    /// # Examples
    ///
    /// Add `user_id` cookie:
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::Cookie;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// # #[allow(unused_variables)]
    /// let req = client.get("/")
    ///     .cookie(Cookie::new("username", "sb"))
    ///     .cookie(Cookie::new("user_id", format!("{}", 12)));
    /// ```
    #[inline]
    pub fn cookie(self, cookie: Cookie<'static>) -> Self {
        self.request.cookies().add(cookie);
        self
    }

    // TODO: For CGI, we want to be able to set the body to be stdin without
    // actually reading everything into a vector. Can we allow that here while
    // keeping the simplicity? Looks like it would require us to reintroduce a
    // NetStream::Local(Box<Read>) or something like that.

    /// Set the body (data) of the request.
    ///
    /// # Examples
    ///
    /// Set the body to be a JSON structure; also sets the Content-Type.
    ///
    /// ```rust
    /// use rocket::local::Client;
    /// use rocket::http::ContentType;
    ///
    /// let client = Client::new(rocket::ignite()).unwrap();
    /// # #[allow(unused_variables)]
    /// let req = client.post("/")
    ///     .header(ContentType::JSON)
    ///     .body(r#"{ "key": "value", "array": [1, 2, 3], }"#);
    /// ```
    #[inline]
    pub fn body<S: AsRef<[u8]>>(mut self, body: S) -> Self {
        self.data = body.as_ref().into();
        self
    }
}

impl<'c> fmt::Debug for LocalRequest<'c> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.request, f)
    }
}

impl<'c> fmt::Debug for LocalResponse<'c> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.response, f)
    }
}

// fn test() {
//     use local::Client;

//     let rocket = Rocket::ignite();
//     let res = {
//         let mut client = Client::new(rocket).unwrap();
//         client.get("/").dispatch()
//     };

//     // let client = Client::new(rocket).unwrap();
//     // let res1 = client.get("/").dispatch();
//     // let res2 = client.get("/").dispatch();
// }

// fn test() {
//     use local::Client;

//     let rocket = Rocket::ignite();
//     let res = {
//         Client::new(rocket).unwrap()
//             .get("/").dispatch();
//     };

//     // let client = Client::new(rocket).unwrap();
//     // let res1 = client.get("/").dispatch();
//     // let res2 = client.get("/").dispatch();
// }

// fn test() {
//     use local::Client;

//     let rocket = Rocket::ignite();
//     let client = Client::new(rocket).unwrap();

//     let res = {
//         let x = client.get("/").dispatch();
//         let y = client.get("/").dispatch();
//     };

//     let x = client;
// }

// fn test() {
//     use local::Client;

//     let rocket1 = Rocket::ignite();
//     let rocket2 = Rocket::ignite();

//     let client1 = Client::new(rocket1).unwrap();
//     let client2 = Client::new(rocket2).unwrap();

//     let res = {
//         let mut res1 = client1.get("/");
//         res1.set_client(&client2);
//         res1
//     };

//     drop(client1);
// }
