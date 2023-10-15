use std::io::Read;
use xmltree::Element;

pub const WADL_NS: &str = "http://wadl.dev.java.net/2009/02";

#[derive(Debug)]
pub struct Application {
    pub resources: Vec<Resource>,
    pub resource_types: Vec<ResourceType>,
    pub docs: Vec<Doc>,
}

#[derive(Debug)]
pub struct Resource {
    pub id: Option<String>,
    pub path: Option<String>,
    pub r#type: Option<String>,
    pub query_type: String,
    pub methods: Vec<Method>,
    pub docs: Vec<Doc>,
    pub subresources: Vec<Resource>,
    pub params: Vec<Param>,
}

#[derive(Debug)]
pub struct Method {
    pub id: String,
    pub name: String,
    pub docs: Vec<Doc>,
    pub request: Option<Request>,
}

#[derive(Debug)]
pub struct Doc {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub content: String,
}

#[derive(Debug)]
pub struct Param {}

#[derive(Debug)]
pub struct Representation {
    pub id: Option<String>,
    pub media_type: Option<String>,
    pub element: Option<String>,
    pub profile: Option<String>,
    pub docs: Vec<Doc>,
    pub params: Vec<Param>,
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
    pub representations: Vec<Representation>,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Xml(xmltree::ParseError),
}

pub fn parse_params(resource_element: &Element) -> Vec<Param> {
    let mut params = Vec::new();

    for param_node in &resource_element.children {
        if let Some(element) = param_node.as_element() {
            if element.name == "param" {
                params.push(Param {});
            }
        }
    }

    params
}

pub fn parse_resource(element: &Element) -> Result<Resource, Error> {
    let id = element.attributes.get("id").cloned();
    let path = element.attributes.get("path").cloned();
    let r#type = element.attributes.get("type").cloned();
    let query_type = element
        .attributes
        .get("queryType")
        .cloned()
        .unwrap_or("application/x-www-form-urlencoded".to_string());

    let docs = parse_docs(element);

    let methods = parse_methods(element);

    let subresources = parse_resources(element)?;

    let params = parse_params(element);

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

pub fn parse_resources(resources_element: &Element) -> Result<Vec<Resource>, Error> {
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

pub fn parse_docs(resource_element: &Element) -> Vec<Doc> {
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

#[derive(Debug)]
pub struct ResourceType {
    pub id: Option<String>,
    pub query_type: String,
    pub methods: Vec<Method>,
    pub docs: Vec<Doc>,
    pub subresources: Vec<Resource>,
    pub params: Vec<Param>,
}

pub fn parse_resource_type(resource_type_element: &Element) -> Result<ResourceType, Error> {
    let id = resource_type_element.attributes.get("id").cloned();
    let query_type = resource_type_element
        .attributes
        .get("queryType")
        .cloned()
        .unwrap_or("application/x-www-form-urlencoded".to_string());

    let docs = parse_docs(resource_type_element);

    let methods = parse_methods(resource_type_element);

    let subresources = parse_resources(resource_type_element)?;

    let params = parse_params(resource_type_element);

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
    let root = Element::parse(reader).map_err(Error::Xml)?;

    for resource_node in &root.children {
        if let Some(element) = resource_node.as_element() {
            if element.name == "resources" {
                resources.extend(parse_resources(element)?);
            } else if element.name == "grammars" {
                for grammar_node in &element.children {
                    if let Some(element) = grammar_node.as_element() {
                        if element.name == "include" {
                            let href = element.attributes.get("href").cloned().unwrap_or_default();
                            todo!();
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

pub fn parse_representations(request_element: &Element) -> Vec<Representation> {
    let mut representations = Vec::new();

    for representation_node in &request_element.children {
        if let Some(element) = representation_node.as_element() {
            if element.name == "representation" {
                let element_name = element.attributes.get("element").cloned();
                let media_type = element.attributes.get("mediaType").cloned();
                let docs = parse_docs(element);
                let id = element.attributes.get("id").cloned();
                let profile = element.attributes.get("profile").cloned();
                let params = parse_params(element);
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

pub fn parse_request(request_element: &Element) -> Request {
    let docs = parse_docs(request_element);

    let params = parse_params(request_element);

    let representations = parse_representations(request_element);

    Request {
        docs,
        params,
        representations,
    }
}

pub fn parse_method(method_element: &Element) -> Method {
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

    let docs = parse_docs(method_element);

    Method {
        id,
        name,
        docs,
        request,
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
