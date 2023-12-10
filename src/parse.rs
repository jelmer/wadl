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

#[test]
fn test_parse_options() {
    let xml = r#"
        <param name="format">
            <option value="json" mediaType="application/json"/>
            <option value="xml" mediaType="application/xml"/>
        </param>
    "#;
    let element = Element::parse(xml.as_bytes()).unwrap();
    let options = parse_options(&element).unwrap();
    assert_eq!(options.len(), 2);
    assert_eq!(
        options.get("json").unwrap(),
        &Some("application/json".parse().unwrap())
    );
    assert_eq!(
        options.get("xml").unwrap(),
        &Some("application/xml".parse().unwrap())
    );
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
                let links = element.children.iter().filter_map(|node| {
                    if let Some(element) = node.as_element() {
                        if element.name == "link" {
                            let resource_type: Option<ResourceTypeRef> = element
                                .attributes
                                .get("resource_type").map(|x| x.parse().unwrap());
                            let relation = element.attributes.get("rel").cloned();
                            let reverse_relation = element.attributes.get("rev").cloned();
                            let doc = parse_docs(element);
                            Some(Link {
                                resource_type,
                                relation,
                                reverse_relation,
                                doc: if doc.len() == 1 {
                                    Some(doc.into_iter().next().unwrap())
                                } else {
                                    assert!(doc.is_empty());
                                    None
                                },
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }).collect::<Vec<_>>();
                let name = element.attributes.get("name").cloned().unwrap();
                let r#type = if let Some(t) = element.attributes.get("type").cloned() {
                    Some(TypeRef::Simple(t))
                } else if !links.is_empty() {
                    Some(TypeRef::ResourceType(links[0].resource_type.clone().unwrap_or(ResourceTypeRef::Empty)))
                } else {
                    None
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
                    links,
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
                use std::io::Write;
                let content = Vec::new();
                let mut cursor = std::io::Cursor::new(content);
                for child in &element.children {
                    match child {
                        xmltree::XMLNode::Text(t) => {
                            cursor.write_all(t.as_bytes()).unwrap();
                        }
                        xmltree::XMLNode::Element(e) => {
                            e.write(&mut cursor).unwrap();
                        }
                        _ => {}
                    };
                }
                let lang = element
                    .attributes
                    .get("lang")
                    .cloned();

                let namespaces = element.namespaces.as_ref();

                let xmlns = namespaces.and_then(|x| x.get("").map(|u| u.parse().unwrap()));

                docs.push(Doc {
                    title,
                    lang,
                    content: String::from_utf8_lossy(cursor.into_inner().as_slice()).to_string(),
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

    let docs = parse_docs(&root);

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
                            href.parse().expect("Invalid URL"),
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

#[test]
fn test_parse_representations() {
    let xml = r#"<response xmlns:xml="http://www.w3.org/XML/1998/namespace">
        <representation id="foo" mediaType="application/json">
            <doc xml:lang="en">Foo</doc>
            <param name="foo" style="plain" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Foo</doc>
            </param>
            <param name="bar" style="query" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Bar</doc>
            </param>
        </representation>
        <representation href='#bar' />
        <representation href="http://example.com/bar" />
        </response>
    "#;

    let root = Element::parse(xml.as_bytes()).unwrap();

    let representations = parse_representations(&root);

    assert_eq!(representations.len(), 3);

    if let Representation::Definition(r) = &representations[0] {
        assert_eq!(r.id, Some("foo".to_string()));
        assert_eq!(r.media_type, Some("application/json".parse().unwrap()));
        assert_eq!(r.docs.len(), 1);
        assert_eq!(r.docs[0].content, "Foo");
        assert_eq!(r.docs[0].lang, Some("en".to_string()));
        assert_eq!(r.params.len(), 2);
        assert_eq!(r.params[0].name, "foo");
        assert_eq!(r.params[0].style, ParamStyle::Plain);
        assert!(r.params[0].required);
        assert_eq!(r.params[0].fixed, Some("baz".to_string()));
        assert_eq!(r.params[0].doc.as_ref().unwrap().content, "Foo");
        assert_eq!(r.params[0].doc.as_ref().unwrap().lang, Some("en".to_string()));
        assert_eq!(r.params[1].name, "bar");
        assert_eq!(r.params[1].style, ParamStyle::Query);
        assert!(r.params[1].required);
        assert_eq!(r.params[1].fixed, Some("baz".to_string()));
    }
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

#[test]
fn test_parses_response() {
    let xml = r#"
        <response status="200">
            <param name="foo" style="plain" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Foo</doc>
            </param>
            <param name="bar" style="query" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Bar</doc>
            </param>
            <param name="baz" style="header" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Baz</doc>
            </param>
            <representation href='#foo' />
            <representation href="http://example.com/bar" />
            <representation mediaType="application/json" />
            <representation element="foo" />
            <representation profile="http://example.com/profile" />
        </response>
    "#;

    let element = Element::parse(xml.as_bytes()).unwrap();

    let response = parse_response(&element);

    assert_eq!(response.status, Some(200));
    assert_eq!(response.representations.len(), 5);
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

#[test]
fn test_parse_request() {
    let xml = r#"
        <request>
            <param name="foo" style="plain" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Foo</doc>
            </param>
            <param name="bar" style="query" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Bar</doc>
            </param>
            <param name="baz" style="header" type="xs:string" required="true" default="bar" fixed="baz">
                <doc xml:lang="en">Baz</doc>
            </param>
            <representation mediaType="application/json" element="foo" profile="bar" id="baz">
                <doc xml:lang="en">Foo</doc>
            </representation>
            <representation href='#qux'/>
        </request>
    "#;

    let element = Element::parse(xml.as_bytes()).unwrap();

    let request = parse_request(&element);

    assert_eq!(request.docs.len(), 0);
    assert_eq!(request.params.len(), 3);
    assert_eq!(request.representations.len(), 2);
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

#[test]
fn test_parse_method() {
    let xml = r#"
        <method name="GET">
            <doc>Get a list of all the widgets</doc>
            <request>
                <doc>Filter the list of widgets</doc>
                <param name="filter" style="query" type="string" required="false">
                    <doc>Filter the list of widgets</doc>
                </param>
            </request>
            <response status="200">
                <doc>Return a list of widgets</doc>
                <representation mediaType="application/json">
                    <doc>Return a list of widgets</doc>
                </representation>
                <param name="id" style="plain" type="string" required="true">
                    <doc>Return a list of widgets</doc>
                </param>
                <param name="name" style="plain" type="string" required="true">
                    <doc>Return a list of widgets</doc>
                </param>

            </response>
        </method>
    "#;

    let method = parse_method(&Element::parse(xml.as_bytes()).unwrap());

    assert_eq!(method.id, "");
    assert_eq!(method.name, "GET");
    assert_eq!(method.docs, vec![Doc { content: "Get a list of all the widgets".to_string(), ..Default::default() }]);
    assert_eq!(method.request.docs, vec![Doc { content: "Filter the list of widgets".to_string(), ..Default::default() }]);
    assert_eq!(method.request.params.len(), 1);
    assert_eq!(method.request.params[0].name, "filter");
    assert_eq!(method.request.params[0].doc.as_ref().unwrap(), &Doc{ content: "Filter the list of widgets".to_string(), ..Default::default() });
    assert_eq!(method.responses.len(), 1);
    assert_eq!(method.responses[0].docs, vec![Doc { content: "Return a list of widgets".to_string(), ..Default::default() }]);
    assert_eq!(method.responses[0].status, Some(200));
    assert_eq!(method.responses[0].representations.len(), 1);
    assert_eq!(method.responses[0].representations[0].as_def().unwrap().media_type, Some("application/json".parse().unwrap()));
    assert_eq!(method.responses[0].params.len(), 2);
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
