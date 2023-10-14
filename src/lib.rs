use std::io::Read;

pub const WADL_NS: &str = "http://wadl.dev.java.net/2009/02";

#[derive(Debug)]
pub struct Application {}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Xml(xml::reader::Error),
}

pub fn parse_wadl<R: Read>(reader: R) -> Result<Application, Error> {
    Ok(Application {})
}
