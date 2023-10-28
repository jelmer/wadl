use crate::ast::*;
use std::collections::HashMap;
use std::io::Read;
use url::Url;
use xmltree::Element;

#[allow(unused)]
pub const WADL_NS: &str = "http://wadl.dev.java.net/2009/02";

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Xml(xmltree::ParseError),
    Url(url::ParseError),
    Mime(mime::FromStrError),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<xmltree::ParseError> for Error {
    fn from(e: xmltree::ParseError) -> Self {
        Error::Xml(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::Url(e)
    }
}

impl From<mime::FromStrError> for Error {
    fn from(e: mime::FromStrError) -> Self {
        Error::Mime(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Xml(e) => write!(f, "XML error: {}", e),
            Error::Url(e) => write!(f, "URL error: {}", e),
            Error::Mime(e) => write!(f, "MIME error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

pub fn parse_options(element: &Element) -> Option<HashMap<String, Option<mime::Mime>>> {
    let mut options = HashMap::new();

    for option_node in &element.children {
        if let Some(element) = option_node.as_element() {
            if element.name == "option" {
                let value = element.attributes.get("value").cloned();
                let media_type = element
                    .attributes
                    .get("mediaType")
                    .cloned()
                    .map(|x| x.parse().unwrap());
                options.insert(value.unwrap(), media_type);
            }
        }
    }

    if options.is_empty() {
        None
    } else {
        Some(options)
    }
}

pub fn parse_params(resource_element: &Element, allowed_styles: &[ParamStyle]) -> Vec<Param> {
    let mut params = Vec::new();

    for param_node in &resource_element.children {
        if let Some(element) = param_node.as_element() {
            if element.name == "param" {
                let style = element
                    .attributes
                    .get("style")
                    .cloned()
                    .map(|s| match s.as_str() {
                        "plain" => ParamStyle::Plain,
                        "matrix" => ParamStyle::Matrix,
                        "query" => ParamStyle::Query,
                        "header" => ParamStyle::Header,
                        "template" => ParamStyle::Template,
                        _ => panic!("Unknown param style: {}", s),
                    })
                    .unwrap();
                let options = parse_options(element);
                let id = element.attributes.get("id").cloned();
                let name = element.attributes.get("name").cloned().unwrap();
                let link_type = element.children.iter().find_map(|node| {
                    if let Some(element) = node.as_element() {
                        if element.name == "link" {
                            match element.attributes.get("resource_type") {
                                Some(href) => Some(TypeRef::ResourceType(
                                    if let Some(s) = href.strip_prefix('#') {
                                        ResourceTypeRef::Id(s.to_string())
                                    } else {
                                        ResourceTypeRef::Link(href.parse().unwrap())
                                    },
                                )),
                                None => Some(TypeRef::ResourceType(ResourceTypeRef::Empty)),
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
                let r#type = if let Some(t) = link_type {
                    Some(t)
                } else {
                    element
                        .attributes
                        .get("type")
                        .map(|t| TypeRef::Simple(t.clone()))
                };
                let path = element.attributes.get("path").cloned();
                let required = element
                    .attributes
                    .get("required")
                    .cloned()
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let repeating = element
                    .attributes
                    .get("repeating")
                    .cloned()
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let fixed = element.attributes.get("fixed").cloned();
                if !allowed_styles.contains(&style) {
                    log::warn!(
                        "Invalid param style: {:?} for element {} (expected one of: {:?})",
                        style,
                        name,
                        allowed_styles
                    );
                }
                let doc = parse_docs(element);
                let r#type = match (r#type, options) {
                    (_, Some(options)) => TypeRef::Options(options),
                    (Some(t), None) => t,
                    (None, None) => TypeRef::NoType,
                };
                params.push(Param {
                    style,
                    id,
                    name,
                    r#type,
                    path,
                    required,
                    repeating,
                    fixed,
                    doc: if doc.len() == 1 {
                        Some(doc.into_iter().next().unwrap())
                    } else {
                        assert!(doc.is_empty());
                        None
                    },
                });
            }
        }
    }

    params
}

fn parse_resource(element: &Element) -> Result<Resource, Error> {
    let id = element.attributes.get("id").cloned();
    let path = element.attributes.get("path").cloned();
    let r#type = element
        .attributes
        .get("type")
        .map(|s| s.as_str())
        .unwrap_or("")
        .split(' ')
        .map(|x| x.parse::<ResourceTypeRef>().unwrap())
        .collect();
    let query_type: mime::Mime = element
        .attributes
        .get("queryType")
        .cloned()
        .unwrap_or("application/x-www-form-urlencoded".to_string())
        .parse()?;

    let docs = parse_docs(element);

    let methods = parse_methods(element);

    let subresources = parse_resources(element)?;

    let params = parse_params(
        element,
        &[
            ParamStyle::Matrix,
            ParamStyle::Query,
            ParamStyle::Header,
            ParamStyle::Template,
        ],
    );

    Ok(Resource {
        id,
        path,
        r#type,
        query_type,
        methods,
        docs,
        subresources,
        params,
    })
}

fn parse_resources(resources_element: &Element) -> Result<Vec<Resource>, Error> {
    let mut resources = Vec::new();

    for resource_node in &resources_element.children {
        if let Some(element) = resource_node.as_element() {
            if element.name == "resource" {
                resources.push(parse_resource(element)?);
            }
        }
    }

    Ok(resources)
}

fn parse_docs(resource_element: &Element) -> Vec<Doc> {
    let mut docs = Vec::new();

    for doc_node in &resource_element.children {
        if let Some(element) = doc_node.as_element() {
            if element.name == "doc" {
                let title = element.attributes.get("title").cloned();
                let content = element.get_text().unwrap_or_default().trim().to_string();
                let lang = element
                    .attributes
                    .get("{http://www.w3.org/XML/1998/namespace}lang")
                    .cloned();
                let xmlns = element
                    .attributes
                    .get("xmlns")
                    .cloned()
                    .map(|u| u.parse().unwrap());

                docs.push(Doc {
                    title,
                    lang,
                    content,
                    xmlns,
                });
            }
        }
    }

    docs
}

fn parse_resource_type(resource_type_element: &Element) -> Result<ResourceType, Error> {
    let id = resource_type_element.attributes.get("id").cloned().unwrap();
    let query_type: mime::Mime = resource_type_element
        .attributes
        .get("queryType")
        .cloned()
        .unwrap_or("application/x-www-form-urlencoded".to_string())
        .parse()?;

    let docs = parse_docs(resource_type_element);

    let methods = parse_methods(resource_type_element);

    let subresources = parse_resources(resource_type_element)?;

    let params = parse_params(
        resource_type_element,
        &[ParamStyle::Header, ParamStyle::Query],
    );

    Ok(ResourceType {
        id,
        query_type,
        methods,
        docs,
        subresources,
        params,
    })
}

pub fn parse<R: Read>(reader: R) -> Result<Application, Error> {
    let mut resources = Vec::new();
    let mut resource_types = Vec::new();
    let mut grammars = Vec::new();
    let root = Element::parse(reader).map_err(Error::Xml)?;

    for resource_node in &root.children {
        if let Some(element) = resource_node.as_element() {
            if element.name == "resources" {
                let more_resources = parse_resources(element)?;
                let base = element.attributes.get("base").cloned();
                resources.push(Resources {
                    base: base.map(|s| s.parse().unwrap()),
                    resources: more_resources,
                });
            } else if element.name == "grammars" {
                for grammar_node in &element.children {
                    if let Some(element) = grammar_node.as_element() {
                        if element.name == "include" {
                            let href: Url = element
                                .attributes
                                .get("href")
                                .cloned()
                                .unwrap()
                                .parse()
                                .unwrap();
                            grammars.push(Grammar { href });
                        }
                    }
                }
            } else if element.name == "resource_type" {
                resource_types.push(parse_resource_type(element)?);
            }
        }
    }

    let docs = parse_docs(&root);

    let representations = parse_representations(&root);

    Ok(Application {
        resources,
        docs,
        resource_types,
        grammars,
        representations: representations
            .into_iter()
            .map(|r| match r {
                Representation::Definition(r) => r,
                Representation::Reference(_) => panic!("Reference in root"),
            })
            .collect(),
    })
}

pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<Application, Error> {
    let file = std::fs::File::open(path).map_err(Error::Io)?;
    parse(file)
}

pub fn parse_string(s: &str) -> Result<Application, Error> {
    parse(s.as_bytes())
}

pub fn parse_bytes(bytes: &[u8]) -> Result<Application, Error> {
    parse(bytes)
}

fn parse_representations(request_element: &Element) -> Vec<Representation> {
    let mut representations = Vec::new();

    for representation_node in &request_element.children {
        if let Some(element) = representation_node.as_element() {
            if element.name == "representation" {
                if let Some(href) = element.attributes.get("href") {
                    if let Some(id) = href.strip_prefix('#') {
                        representations.push(Representation::Reference(RepresentationRef::Id(
                            id.to_string(),
                        )));
                    } else {
                        representations.push(Representation::Reference(RepresentationRef::Link(
                            href.parse().unwrap(),
                        )));
                    }
                } else {
                    let element_name = element.attributes.get("element").cloned();
                    let media_type = element
                        .attributes
                        .get("mediaType")
                        .map(|s| s.parse().unwrap());
                    let docs = parse_docs(element);
                    let id = element.attributes.get("id").cloned();
                    let profile = element.attributes.get("profile").cloned();
                    let params = parse_params(element, &[ParamStyle::Plain, ParamStyle::Query]);
                    representations.push(Representation::Definition(RepresentationDef {
                        id,
                        media_type,
                        docs,
                        element: element_name,
                        profile,
                        params,
                    }));
                }
            }
        }
    }

    representations
}

fn parse_response(response_element: &Element) -> Response {
    let docs = parse_docs(response_element);

    let representations = parse_representations(response_element);

    let status = response_element
        .attributes
        .get("status")
        .map(|s| s.parse().unwrap());

    let params = parse_params(response_element, &[ParamStyle::Header]);

    Response {
        docs,
        params,
        status,
        representations,
    }
}

fn parse_request(request_element: &Element) -> Request {
    let docs = parse_docs(request_element);

    let params = parse_params(request_element, &[ParamStyle::Header, ParamStyle::Query]);

    let representations = parse_representations(request_element);

    Request {
        docs,
        params,
        representations,
    }
}

fn parse_method(method_element: &Element) -> Method {
    let id = method_element
        .attributes
        .get("id")
        .cloned()
        .unwrap_or_default();
    let name = method_element
        .attributes
        .get("name")
        .cloned()
        .unwrap_or_default();

    let request_element = method_element
        .children
        .iter()
        .find(|node| node.as_element().map_or(false, |e| e.name == "request"))
        .and_then(|node| node.as_element());

    let request = request_element.map(parse_request).unwrap_or_default();

    let responses = method_element
        .children
        .iter()
        .filter(|node| node.as_element().map_or(false, |e| e.name == "response"))
        .map(|node| node.as_element().unwrap())
        .map(parse_response)
        .collect();

    let docs = parse_docs(method_element);

    Method {
        id,
        name,
        docs,
        request,
        responses,
    }
}

fn parse_methods(resource_element: &Element) -> Vec<Method> {
    let mut methods = Vec::new();

    for method_node in &resource_element.children {
        if let Some(element) = method_node.as_element() {
            if element.name == "method" {
                methods.push(parse_method(element));
            }
        }
    }

    methods
}
