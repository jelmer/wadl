use std::collections::HashMap;
use std::io::Read;
use url::Url;
use xmltree::Element;

pub const WADL_NS: &str = "http://wadl.dev.java.net/2009/02";

pub type Id = String;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParamStyle {
    Plain,
    Matrix,
    Query,
    Header,
    Template,
}

/// A WADL application.
#[derive(Debug)]
pub struct Application {
    /// Resources defined at the application level.
    pub resources: Vec<Resources>,

    pub resource_types: Vec<ResourceType>,

    /// Documentation for the application.
    pub docs: Vec<Doc>,

    pub grammars: Vec<Grammar>,
}

impl std::str::FromStr for Application {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_string(s)
    }
}

#[derive(Debug)]
pub struct Resources {
    /// The base URL for the resources.
    pub base: Option<Url>,

    /// The resources defined at this level.
    pub resources: Vec<Resource>,
}

#[derive(Debug)]
pub struct Grammar {
    pub href: Url,
}

#[derive(Debug)]
pub enum TypeRef {
    Name(String),
    Id(Id),
    Link(Url),
}

impl std::str::FromStr for TypeRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('#') {
            Ok(TypeRef::Id(s[1..].to_string()))
        } else {
            Ok(TypeRef::Name(s.to_string()))
        }
    }
}

#[derive(Debug)]
pub struct Resource {
    /// The ID of the resource.
    pub id: Option<Id>,

    /// The path of the resource.
    pub path: Option<String>,

    /// The type of the resource.
    pub r#type: Option<Vec<TypeRef>>,

    /// The query type of the resource.
    pub query_type: mime::Mime,

    /// The methods defined at this level.
    pub methods: Vec<Method>,

    /// The docs for the resource.
    pub docs: Vec<Doc>,

    /// Sub-resources of this resource.
    pub subresources: Vec<Resource>,

    /// The params for this resource.
    pub params: Vec<Param>,
}

#[derive(Debug)]
pub struct Method {
    pub id: Id,
    pub name: String,
    pub docs: Vec<Doc>,
    pub request: Option<Request>,
    pub responses: Vec<Response>,
}

#[derive(Debug)]
pub struct Doc {
    /// The title of the documentation.
    pub title: Option<String>,

    /// The language of the documentation.
    pub lang: Option<String>,

    /// The content of the documentation.
    pub content: String,
}

#[derive(Debug)]
pub struct Param {
    pub style: ParamStyle,
    pub options: Option<HashMap<String, Option<String>>>,
    pub id: Option<Id>,
    pub name: String,
    pub r#type: TypeRef,
    pub path: Option<String>,
    pub required: bool,
    pub repeating: bool,
    pub fixed: Option<String>,
}

#[derive(Debug)]
pub struct Representation {
    pub id: Option<Id>,
    pub media_type: Option<mime::Mime>,
    pub element: Option<String>,
    pub profile: Option<String>,
    pub docs: Vec<Doc>,
    pub params: Vec<Param>,
}

impl Representation {
    pub fn url(&self, base_url: &Url) -> Option<Url> {
        if let Some(id) = &self.id {
            let mut url = base_url.clone();
            url.set_fragment(Some(id));
            Some(url)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Request {
    pub docs: Vec<Doc>,
    pub params: Vec<Param>,
    pub representations: Vec<Representation>,
}

#[derive(Debug)]
pub struct Response {
    pub docs: Vec<Doc>,
    pub params: Vec<Param>,
    pub status: Option<i32>,
    pub representations: Vec<Representation>,
}

#[derive(Debug)]
pub struct ResourceType {
    pub id: Option<Id>,
    pub query_type: mime::Mime,
    pub methods: Vec<Method>,
    pub docs: Vec<Doc>,
    pub subresources: Vec<Resource>,
    pub params: Vec<Param>,
}

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

pub fn parse_options(element: &Element) -> Option<HashMap<String, Option<String>>> {
    let mut options = HashMap::new();

    for option_node in &element.children {
        if let Some(element) = option_node.as_element() {
            if element.name == "option" {
                let value = element.attributes.get("value").cloned();
                let media_type = element.attributes.get("mediaType").cloned();
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
                let r#type = if let Some(t) = element.attributes.get("type") {
                    Some(TypeRef::Name(t.clone()))
                } else {
                    element.children.iter().find_map(|node| {
                        if let Some(element) = node.as_element() {
                            if element.name == "link" {
                                let href =
                                    element.attributes.get("resource_type").cloned().unwrap();
                                Some(TypeRef::Link(href.parse().unwrap()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
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
                let r#type = match r#type {
                    Some(t) => t,
                    None => {
                        log::warn!("No type for param: {}", name);
                        TypeRef::Name("string".to_string())
                    }
                };
                params.push(Param {
                    options,
                    style,
                    id,
                    name,
                    r#type,
                    path,
                    required,
                    repeating,
                    fixed,
                });
            }
        }
    }

    params
}

fn parse_resource(element: &Element) -> Result<Resource, Error> {
    let id = element.attributes.get("id").cloned();
    let path = element.attributes.get("path").cloned();
    let r#type = element.attributes.get("type").cloned().map(|s| {
        s.split(" ")
            .map(|x| x.parse::<TypeRef>().unwrap())
            .collect()
    });
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
                docs.push(Doc {
                    title,
                    lang,
                    content,
                });
            }
        }
    }

    docs
}

fn parse_resource_type(resource_type_element: &Element) -> Result<ResourceType, Error> {
    let id = resource_type_element.attributes.get("id").cloned();
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

    Ok(Application {
        resources,
        docs,
        resource_types,
        grammars,
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
                let element_name = element.attributes.get("element").cloned();
                let media_type = element
                    .attributes
                    .get("mediaType")
                    .map(|s| s.parse().unwrap());
                let docs = parse_docs(element);
                let id = element.attributes.get("id").cloned();
                let profile = element.attributes.get("profile").cloned();
                let params = parse_params(element, &[ParamStyle::Plain, ParamStyle::Query]);
                representations.push(Representation {
                    id,
                    media_type,
                    docs,
                    element: element_name,
                    profile,
                    params,
                });
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

    let request = request_element.map(parse_request);

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
