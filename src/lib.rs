#![deny(missing_docs)]
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

/// A client for a WADL API
pub trait Client {
    /// Create a new request builder
    fn request(&self, method: reqwest::Method, url: url::Url) -> reqwest::blocking::RequestBuilder;
}

impl Client for reqwest::blocking::Client {
    fn request(&self, method: reqwest::Method, url: url::Url) -> reqwest::blocking::RequestBuilder {
        self.request(method, url)
    }
}

#[cfg(feature = "async")]
/// Asynchronous features
pub mod r#async {
    use super::*;

    /// A client for a WADL API
    #[async_trait::async_trait]
    pub trait Client: Sync + Send {
        /// Create a new request builder
        async fn request(&self, method: reqwest::Method, url: url::Url) -> reqwest::RequestBuilder;
    }

    #[async_trait::async_trait]
    impl Client for reqwest::Client {
        async fn request(&self, method: reqwest::Method, url: url::Url) -> reqwest::RequestBuilder {
            self.request(method, url)
        }
    }

    /// Get the WADL AST from a URL.
    pub async fn get_wadl_resource_by_href(
        client: &dyn Client,
        href: &url::Url,
    ) -> Result<crate::ast::Resource, Error> {
        let mut req = client.request(reqwest::Method::GET, href.clone()).await;

        req = req.header(reqwest::header::ACCEPT, super::WADL_MIME_TYPE);

        let res = req.send().await?;

        let text = res.text().await?;

        let application = super::parse_string(&text)?;

        let resource = application.get_resource_by_href(href).unwrap();

        Ok(resource.clone())
    }
}

#[derive(Debug)]
/// The error type for this crate.
pub enum Error {
    /// The URL is invalid.
    InvalidUrl,

    /// A reqwest error occurred.
    Reqwest(reqwest::Error),

    /// The URL could not be parsed.
    Url(url::ParseError),

    /// The JSON could not be parsed.
    Json(serde_json::Error),

    /// The WADL could not be parsed.
    Wadl(ParseError),

    /// The response status was not handled by the library.
    UnhandledStatus(reqwest::StatusCode),

    /// The response content type was not handled by the library.
    UnhandledContentType(Option<mime::Mime>),

    /// An I/O error occurred.
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
            Error::UnhandledContentType(Some(c)) => write!(f, "Unhandled content type: {}", c),
            Error::UnhandledContentType(None) => write!(f, "No content type"),
            Error::UnhandledStatus(s) => write!(f, "Unhandled status: {}", s),
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

/// Get the WADL AST from a URL.
pub fn get_wadl_resource_by_href(
    client: &dyn Client,
    href: &url::Url,
) -> Result<crate::ast::Resource, Error> {
    let mut req = client.request(reqwest::Method::GET, href.clone());

    req = req.header(reqwest::header::ACCEPT, WADL_MIME_TYPE);

    let res = req.send()?;

    let text = res.text()?;

    let application = parse_string(&text)?;

    let resource = application.get_resource_by_href(href).unwrap();

    Ok(resource.clone())
}
