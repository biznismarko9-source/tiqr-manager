//! TIQR in a phone's browser: `reqwest::blocking`, as far as the shared code
//! uses it.
//!
//! On WebAssembly `reqwest` is this crate (lib.rs: `extern crate ... as
//! reqwest`), so the shared code's HTTP calls compile unchanged. Sending is
//! the phone build's business: it sets a transport (`set_transport`); until it
//! does, every request answers with `NO_TRANSPORT`, which the shared code
//! shows the way it shows any connection that failed.
use std::fmt;
use std::sync::Mutex;
use std::time::Duration;

pub use http::{header, Method, StatusCode};
pub use url::Url;

use http::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};

/// What a request answers before the phone build has a way to send it.
pub const NO_TRANSPORT: &str = "This needs an internet connection TIQR on the phone cannot make yet.";

/// What a client that must reach only checked addresses answers.
pub const PINNED_REFUSED: &str = "TIQR on the phone cannot open this safely - open it on the computer.";

#[derive(Debug)]
pub struct Error {
    message: String,
    status: Option<StatusCode>,
}

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Error { message: message.into(), status: None }
    }

    pub fn is_timeout(&self) -> bool {
        self.message.contains("timed out")
    }

    pub fn is_connect(&self) -> bool {
        self.status.is_none() && !self.is_timeout()
    }

    pub fn is_status(&self) -> bool {
        self.status.is_some()
    }

    pub fn is_request(&self) -> bool {
        self.status.is_none()
    }

    pub fn is_decode(&self) -> bool {
        false
    }

    pub fn is_body(&self) -> bool {
        false
    }

    pub fn status(&self) -> Option<StatusCode> {
        self.status
    }

    pub fn url(&self) -> Option<&Url> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub mod redirect {
    #[derive(Clone, Copy, Debug)]
    pub struct Policy {
        pub(crate) follow: bool,
    }

    impl Policy {
        pub fn none() -> Self {
            Policy { follow: false }
        }

        pub fn limited(_max: usize) -> Self {
            Policy { follow: true }
        }
    }
}

pub trait IntoUrl {
    fn into_url(self) -> Result<Url>;
}

impl IntoUrl for Url {
    fn into_url(self) -> Result<Url> {
        Ok(self)
    }
}

impl IntoUrl for &Url {
    fn into_url(self) -> Result<Url> {
        Ok(self.clone())
    }
}

impl IntoUrl for &str {
    fn into_url(self) -> Result<Url> {
        Url::parse(self).map_err(|e| Error::new(format!("builder error: {e}")))
    }
}

impl IntoUrl for String {
    fn into_url(self) -> Result<Url> {
        self.as_str().into_url()
    }
}

impl IntoUrl for &String {
    fn into_url(self) -> Result<Url> {
        self.as_str().into_url()
    }
}

/// A request, as the phone build's transport receives it.
pub struct OutgoingRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: Option<u64>,
    pub follow_redirects: bool,
}

/// The answer the transport hands back.
pub struct IncomingResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub url: String,
}

type Transport = Box<dyn Fn(OutgoingRequest) -> std::result::Result<IncomingResponse, String> + Send + Sync>;
static TRANSPORT: Mutex<Option<Transport>> = Mutex::new(None);

/// How requests leave the phone build.
pub fn set_transport(
    transport: impl Fn(OutgoingRequest) -> std::result::Result<IncomingResponse, String> + Send + Sync + 'static,
) {
    *TRANSPORT.lock().unwrap() = Some(Box::new(transport));
}

pub mod blocking {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    #[derive(Clone)]
    pub struct Client {
        timeout: Option<Duration>,
        follow: bool,
        headers: HeaderMap,
    }

    impl Default for Client {
        fn default() -> Self {
            Client::new()
        }
    }

    impl Client {
        pub fn new() -> Client {
            Client { timeout: None, follow: true, headers: HeaderMap::new() }
        }

        pub fn builder() -> ClientBuilder {
            ClientBuilder { client: Client::new(), refused: None }
        }

        pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
            self.request(Method::GET, url)
        }

        pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
            self.request(Method::POST, url)
        }

        pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
            self.request(Method::PUT, url)
        }

        pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
            self.request(Method::PATCH, url)
        }

        pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
            self.request(Method::DELETE, url)
        }

        pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
            self.request(Method::HEAD, url)
        }

        pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
            RequestBuilder {
                follow: self.follow,
                method,
                url: url.into_url(),
                headers: self.headers.clone(),
                body: None,
                timeout: self.timeout,
                error: None,
            }
        }
    }

    pub struct ClientBuilder {
        client: Client,
        refused: Option<&'static str>,
    }

    impl ClientBuilder {
        pub fn timeout(mut self, timeout: Duration) -> Self {
            self.client.timeout = Some(timeout);
            self
        }

        pub fn connect_timeout(self, _timeout: Duration) -> Self {
            self
        }

        pub fn redirect(mut self, policy: redirect::Policy) -> Self {
            self.client.follow = policy.follow;
            self
        }

        /// The browser picks its own addresses, so a client that must reach
        /// only the ones that were checked (pictures and links in mail, which
        /// must never reach a private network) is refused rather than sent
        /// without that check.
        pub fn resolve(mut self, _domain: &str, _addr: std::net::SocketAddr) -> Self {
            self.refused = Some(PINNED_REFUSED);
            self
        }

        pub fn resolve_to_addrs(mut self, _domain: &str, _addrs: &[std::net::SocketAddr]) -> Self {
            self.refused = Some(PINNED_REFUSED);
            self
        }

        /// The browser decides about proxies.
        pub fn no_proxy(self) -> Self {
            self
        }

        pub fn user_agent<V>(mut self, value: V) -> Self
        where
            HeaderValue: TryFrom<V>,
        {
            if let Ok(value) = HeaderValue::try_from(value) {
                self.client.headers.insert(USER_AGENT, value);
            }
            self
        }

        pub fn default_headers(mut self, headers: HeaderMap) -> Self {
            for (name, value) in headers.iter() {
                self.client.headers.insert(name.clone(), value.clone());
            }
            self
        }

        pub fn build(self) -> Result<Client> {
            match self.refused {
                Some(why) => Err(Error::new(why)),
                None => Ok(self.client),
            }
        }
    }

    /// A request body.
    pub struct Body(Vec<u8>);

    impl From<Vec<u8>> for Body {
        fn from(bytes: Vec<u8>) -> Self {
            Body(bytes)
        }
    }

    impl From<String> for Body {
        fn from(text: String) -> Self {
            Body(text.into_bytes())
        }
    }

    impl From<&'static str> for Body {
        fn from(text: &'static str) -> Self {
            Body(text.as_bytes().to_vec())
        }
    }

    impl From<&'static [u8]> for Body {
        fn from(bytes: &'static [u8]) -> Self {
            Body(bytes.to_vec())
        }
    }

    impl From<bytes::Bytes> for Body {
        fn from(bytes: bytes::Bytes) -> Self {
            Body(bytes.to_vec())
        }
    }

    pub struct RequestBuilder {
        follow: bool,
        method: Method,
        url: Result<Url>,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
        timeout: Option<Duration>,
        error: Option<Error>,
    }

    impl RequestBuilder {
        pub fn header<K, V>(mut self, key: K, value: V) -> Self
        where
            HeaderName: TryFrom<K>,
            HeaderValue: TryFrom<V>,
        {
            match (HeaderName::try_from(key), HeaderValue::try_from(value)) {
                (Ok(name), Ok(value)) => {
                    self.headers.append(name, value);
                }
                _ => self.error = Some(Error::new("builder error: a header could not be written")),
            }
            self
        }

        pub fn headers(mut self, headers: HeaderMap) -> Self {
            for (name, value) in headers.iter() {
                self.headers.insert(name.clone(), value.clone());
            }
            self
        }

        pub fn bearer_auth<T: fmt::Display>(self, token: T) -> Self {
            self.header(AUTHORIZATION, format!("Bearer {token}"))
        }

        pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> Self {
            match serde_urlencoded::to_string(query) {
                Ok(extra) => {
                    if let Ok(url) = &mut self.url {
                        if !extra.is_empty() {
                            let joined = match url.query() {
                                Some(old) if !old.is_empty() => format!("{old}&{extra}"),
                                _ => extra,
                            };
                            url.set_query(Some(&joined));
                        }
                    }
                }
                Err(e) => self.error = Some(Error::new(format!("builder error: {e}"))),
            }
            self
        }

        pub fn form<T: Serialize + ?Sized>(mut self, form: &T) -> Self {
            match serde_urlencoded::to_string(form) {
                Ok(body) => {
                    self.headers
                        .insert(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"));
                    self.body = Some(body.into_bytes());
                }
                Err(e) => self.error = Some(Error::new(format!("builder error: {e}"))),
            }
            self
        }

        pub fn json<T: Serialize + ?Sized>(mut self, json: &T) -> Self {
            match serde_json::to_vec(json) {
                Ok(body) => {
                    if !self.headers.contains_key(CONTENT_TYPE) {
                        self.headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                    }
                    self.body = Some(body);
                }
                Err(e) => self.error = Some(Error::new(format!("builder error: {e}"))),
            }
            self
        }

        pub fn body<B: Into<Body>>(mut self, body: B) -> Self {
            self.body = Some(body.into().0);
            self
        }

        pub fn timeout(mut self, timeout: Duration) -> Self {
            self.timeout = Some(timeout);
            self
        }

        pub fn send(self) -> Result<Response> {
            if let Some(error) = self.error {
                return Err(error);
            }
            let url = self.url?;
            let request = OutgoingRequest {
                method: self.method.to_string(),
                url: url.to_string(),
                headers: self
                    .headers
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                    .collect(),
                body: self.body,
                timeout_ms: self.timeout.map(|d| d.as_millis() as u64),
                follow_redirects: self.follow,
            };
            let transport = TRANSPORT.lock().unwrap();
            let Some(send) = transport.as_ref() else {
                return Err(Error::new(NO_TRANSPORT));
            };
            let answer = send(request).map_err(Error::new)?;
            let mut headers = HeaderMap::new();
            for (name, value) in answer.headers {
                if let (Ok(name), Ok(value)) = (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(&value)) {
                    headers.append(name, value);
                }
            }
            Ok(Response {
                status: StatusCode::from_u16(answer.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                headers,
                body: answer.body,
                read: 0,
                url: Url::parse(&answer.url).unwrap_or(url),
            })
        }
    }

    pub struct Response {
        status: StatusCode,
        headers: HeaderMap,
        body: Vec<u8>,
        /// How much of `body` `Read` has handed out.
        read: usize,
        url: Url,
    }

    /// As reqwest's: the body, read like a file (`std::io::copy`).
    impl std::io::Read for Response {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let rest = &self.body[self.read..];
            let n = rest.len().min(buf.len());
            buf[..n].copy_from_slice(&rest[..n]);
            self.read += n;
            Ok(n)
        }
    }

    impl Response {
        pub fn status(&self) -> StatusCode {
            self.status
        }

        pub fn headers(&self) -> &HeaderMap {
            &self.headers
        }

        pub fn url(&self) -> &Url {
            &self.url
        }

        pub fn content_length(&self) -> Option<u64> {
            Some(self.body.len() as u64)
        }

        /// What `Read` has not handed out yet - all of it, usually.
        fn rest(mut self) -> Vec<u8> {
            self.body.split_off(self.read)
        }

        pub fn text(self) -> Result<String> {
            Ok(String::from_utf8_lossy(&self.rest()).into_owned())
        }

        pub fn bytes(self) -> Result<bytes::Bytes> {
            Ok(bytes::Bytes::from(self.rest()))
        }

        pub fn json<T: DeserializeOwned>(self) -> Result<T> {
            serde_json::from_slice(&self.rest()).map_err(|e| Error::new(format!("error decoding response body: {e}")))
        }

        pub fn error_for_status(self) -> Result<Self> {
            if self.status.is_client_error() || self.status.is_server_error() {
                Err(Error { message: format!("HTTP status {}", self.status), status: Some(self.status) })
            } else {
                Ok(self)
            }
        }
    }

    pub fn get<U: IntoUrl>(url: U) -> Result<Response> {
        Client::new().get(url).send()
    }
}
