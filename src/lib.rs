pub mod ast;
#[cfg(feature = "codegen")]
pub mod codegen;
mod parse;

pub const WADL_MIME_TYPE: &str = "application/vnd.sun.wadl+xml";

pub use parse::{parse, parse_bytes, parse_file, parse_string, Error as ParseError};

use url::Url;

/// The root of the web service.
pub trait Resource {
    /// The URL of the resource
    fn url(&self) -> &Url;
}

pub trait Client {
    fn request(&self, method: reqwest::Method, url: url::Url) -> reqwest::blocking::RequestBuilder;
}

impl Client for reqwest::blocking::Client {
    fn request(&self, method: reqwest::Method, url: url::Url) -> reqwest::blocking::RequestBuilder {
        self.request(method, url)
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidUrl,
    Reqwest(reqwest::Error),
    Url(url::ParseError),
    Json(serde_json::Error),
    Wadl(ParseError),
    UnhandledResponse(reqwest::blocking::Response),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidUrl => write!(f, "Invalid URL"),
            Error::Reqwest(err) => write!(f, "Reqwest error: {}", err),
            Error::Url(err) => write!(f, "URL error: {}", err),
            Error::Json(err) => write!(f, "JSON error: {}", err),
            Error::Wadl(err) => write!(f, "WADL error: {}", err),
            Error::UnhandledResponse(res) => write!(f, "Unhandled response. Code: {}, response type: {}", res.status(), res.headers().get("content-type").unwrap_or(&reqwest::header::HeaderValue::from_static("unknown")).to_str().unwrap_or("unknown")),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::Url(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::Wadl(err)
    }
}
