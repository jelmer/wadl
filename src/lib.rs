//! # WADL
//!
//! A crate for parsing WADL files and generating Rust code from them.

pub mod ast;
#[cfg(feature = "codegen")]
pub mod codegen;
mod parse;

/// The MIME type of WADL files.
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

    /// The URL could not be parsed.
    Url(url::ParseError),

    /// The JSON could not be parsed.
    Json(serde_json::Error),

    /// The WADL could not be parsed.
    Wadl(ParseError),

    /// The response status was not handled by the library.
    UnhandledStatus(reqwest::blocking::Response),

    /// The response content type was not handled by the library.
    UnhandledContentType(reqwest::blocking::Response),

    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
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
            Error::UnhandledStatus(res) => write!(f, "Unhandled response. Code: {}, response type: {}", res.status(), res.headers().get("content-type").unwrap_or(&reqwest::header::HeaderValue::from_static("unknown")).to_str().unwrap_or("unknown")),
            Error::UnhandledContentType(res) => write!(f, "Unhandled response content type: {}", res.headers().get("content-type").unwrap_or(&reqwest::header::HeaderValue::from_static("unknown")).to_str().unwrap_or("unknown")),
            Error::Io(err) => write!(f, "IO error: {}", err),
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
