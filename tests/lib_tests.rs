use std::error::Error as StdError;
use wadl::Error;

#[test]
fn test_error_display() {
    let error = Error::InvalidUrl;
    let display_str = format!("{}", error);
    assert!(display_str.contains("Invalid URL"));
}

#[test]
fn test_error_source() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let error = Error::Io(io_error);
    assert!(StdError::source(&error).is_some());

    let invalid_url_error = Error::InvalidUrl;
    assert!(StdError::source(&invalid_url_error).is_none());
}

#[test]
fn test_error_from_io() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let error: Error = io_error.into();
    match error {
        Error::Io(_) => assert!(true),
        _ => assert!(false, "Expected IO error"),
    }
}

#[test]
fn test_error_from_url() {
    let url_error = url::ParseError::EmptyHost;
    let error: Error = url_error.into();
    match error {
        Error::Url(_) => assert!(true),
        _ => assert!(false, "Expected URL error"),
    }
}

#[test]
fn test_error_debug() {
    let error = Error::InvalidUrl;
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("InvalidUrl"));
}

#[test]
fn test_multiple_error_types() {
    // Test different error variants
    let invalid_url_error = Error::InvalidUrl;
    let io_error = Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "io error"));
    let url_error = Error::Url(url::ParseError::EmptyHost);

    assert!(format!("{}", invalid_url_error).contains("Invalid URL"));
    assert!(format!("{}", io_error).contains("io error"));
    assert!(format!("{}", url_error).len() > 0);
}
