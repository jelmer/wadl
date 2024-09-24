//! Abstract syntax tree for WADL documents.
use iri_string::spec::IriSpec;
use iri_string::types::RiReferenceString;
use std::collections::HashMap;
use url::Url;

/// Identifier for a resource, method, parameter, etc.
pub type Id = String;

/// Parameter style
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParamStyle {
    /// Specifies a component of the representation formatted as a string encoding of the parameter value according to the rules of the media type.
    Plain,

    /// Specifies a matrix URI component.
    Matrix,

    /// Specifies a URI query parameter represented according to the rules for the query component media type specified by the queryType attribute.
    Query,

    /// Specifies a HTTP header that pertains to the HTTP request (resource or request) or HTTP response (response)
    Header,

    /// The parameter is represented as a string encoding of the parameter value and is substituted into the value of the path attribute of the resource element as described in section 2.6.1.
    Template,
}

/// A WADL application.
#[derive(Debug)]
pub struct Application {
    /// Resources defined at the application level.
    pub resources: Vec<Resources>,

    /// Resource types defined at the application level.
    pub resource_types: Vec<ResourceType>,

    /// Documentation for the application.
    pub docs: Vec<Doc>,

    /// List of grammars
    pub grammars: Vec<Grammar>,

    /// Representations defined at the application level.
    pub representations: Vec<RepresentationDef>,
}

impl Application {
    /// Get a resource type by its ID.
    pub fn get_resource_type_by_id(&self, id: &str) -> Option<&ResourceType> {
        self.resource_types.iter().find(|rt| id == rt.id.as_str())
    }

    /// Get a resource type by its href, which may be a fragment or a full URL.
    pub fn get_resource_type_by_href(&self, href: &Url) -> Option<&ResourceType> {
        // TODO(jelmer): Check that href matches us?
        if let Some(fragment) = href.fragment() {
            self.get_resource_type_by_id(fragment)
        } else {
            None
        }
    }

    /// Iterate over all resources defined in this application.
    pub fn iter_resources(&self) -> impl Iterator<Item = (Url, &Resource)> {
        self.resources
            .iter()
            .flat_map(|rs| rs.resources.iter().map(|r| (r.url(rs.base.as_ref()), r)))
    }

    /// Get a resource by its ID.
    pub fn get_resource_by_href(&self, href: &Url) -> Option<&Resource> {
        self.iter_resources()
            .find(|(url, _)| url == href)
            .map(|(_, r)| r)
    }

    /// Iterate over all types defined in this application.
    pub fn iter_referenced_types(&self) -> impl Iterator<Item = String> + '_ {
        self.iter_resources()
            .flat_map(|(_u, r)| r.iter_referenced_types())
            .chain(
                self.resource_types
                    .iter()
                    .flat_map(|rt| rt.iter_referenced_types()),
            )
    }

    /// Iterate over all parameters defined in this application.
    pub fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        self.iter_resources()
            .flat_map(|(_u, r)| r.iter_all_params())
            .chain(
                self.resource_types
                    .iter()
                    .flat_map(|rt| rt.iter_all_params()),
            )
            .chain(
                self.representations
                    .iter()
                    .flat_map(|r| r.iter_all_params()),
            )
    }
}

impl std::str::FromStr for Application {
    type Err = crate::parse::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse::parse_string(s)
    }
}

#[derive(Debug)]
/// A collection of resources.
pub struct Resources {
    /// The base URL for the resources.
    pub base: Option<Url>,

    /// The resources defined at this level.
    pub resources: Vec<Resource>,
}

#[derive(Debug)]
/// A grammar
pub struct Grammar {
    /// The href of the grammar.
    pub href: RiReferenceString<IriSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A reference to a resource type.
pub enum ResourceTypeRef {
    /// A reference to a resource type defined in the same document.
    Id(Id),

    /// A reference to a resource type defined in another document.
    Link(Url),

    /// An empty reference.
    Empty,
}

impl std::str::FromStr for ResourceTypeRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(ResourceTypeRef::Empty),
            s => {
                if let Some(s) = s.strip_prefix('#') {
                    Ok(ResourceTypeRef::Id(s.to_string()))
                } else {
                    Ok(ResourceTypeRef::Link(
                        s.parse().map_err(|e| format!("{}", e))?,
                    ))
                }
            }
        }
    }
}

#[test]
fn parse_resource_type_ref() {
    use crate::ast::ResourceTypeRef::*;
    use std::str::FromStr;
    assert_eq!(Empty, ResourceTypeRef::from_str("").unwrap());
    assert_eq!(
        Id("id".to_owned()),
        ResourceTypeRef::from_str("#id").unwrap()
    );
    assert_eq!(
        Link(Url::parse("https://example.com").unwrap()),
        ResourceTypeRef::from_str("https://example.com").unwrap()
    );
}

impl ResourceTypeRef {
    /// Return the ID of the resource type reference.
    pub fn id(&self) -> Option<&str> {
        match self {
            ResourceTypeRef::Id(id) => Some(id),
            ResourceTypeRef::Link(l) => l.fragment(),
            ResourceTypeRef::Empty => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// An option element defines one of a set of possible values for the parameter represented by its parent param element.
pub struct Options(HashMap<String, Option<mime::Mime>>);

impl std::hash::Hash for Options {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut items = self.0.iter().collect::<Vec<_>>();
        items.sort();
        for (key, value) in items {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl Options {
    /// Create a new options object
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Number of items in this Options
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate over all items in this Options
    pub fn iter(&self) -> impl Iterator<Item = (&str, Option<&mime::Mime>)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_ref()))
    }

    /// Return an iterator over all keys
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(|k| k.as_str())
    }

    /// Check if this Options is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Insert a new key-value pair into this Options
    pub fn insert(&mut self, key: String, value: Option<mime::Mime>) {
        self.0.insert(key, value);
    }

    /// Get the value for a key
    pub fn get(&self, key: &str) -> Option<&Option<mime::Mime>> {
        self.0.get(key)
    }
}

impl From<Vec<String>> for Options {
    fn from(v: Vec<String>) -> Self {
        Self(v.into_iter().map(|s| (s, None)).collect())
    }
}

impl From<Vec<&str>> for Options {
    fn from(v: Vec<&str>) -> Self {
        Self(v.into_iter().map(|s| (s.to_string(), None)).collect())
    }
}

#[derive(Debug, Clone)]
/// A resource
pub struct Resource {
    /// The ID of the resource.
    pub id: Option<Id>,

    /// The path of the resource.
    pub path: Option<String>,

    /// The types of the resource.
    pub r#type: Vec<ResourceTypeRef>,

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

impl Resource {
    /// Get the URL of this resource.
    pub fn url(&self, base_url: Option<&Url>) -> Url {
        if let Some(base_url) = base_url {
            base_url.join(self.path.as_ref().unwrap()).unwrap()
        } else {
            Url::parse(self.path.as_ref().unwrap()).unwrap()
        }
    }

    /// Iterate over all parameters defined in this resource.
    pub(crate) fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        let mut params = self.params.iter().collect::<Vec<_>>();

        params.extend(self.subresources.iter().flat_map(|r| r.iter_all_params()));
        params.extend(self.methods.iter().flat_map(|m| m.iter_all_params()));

        params.into_iter()
    }

    /// Iterate over all types referenced by this resource.
    pub fn iter_referenced_types(&self) -> impl Iterator<Item = String> + '_ {
        self.iter_all_params().map(|p| p.r#type.clone())
    }
}

#[test]
fn test_resource_url() {
    let r = Resource {
        id: None,
        path: Some("/foo".to_string()),
        r#type: vec![],
        query_type: mime::APPLICATION_JSON,
        methods: vec![],
        docs: vec![],
        subresources: vec![],
        params: vec![],
    };
    assert_eq!(
        r.url(Some(&Url::parse("http://example.com").unwrap())),
        Url::parse("http://example.com/foo").unwrap()
    );
    assert_eq!(
        r.url(Some(&Url::parse("http://example.com/bar").unwrap())),
        Url::parse("http://example.com/foo").unwrap()
    );
}

#[derive(Debug, Clone)]
/// A HTTP Method
pub struct Method {
    /// Identifier of this method
    pub id: Id,

    /// The name of the method.
    pub name: String,

    /// The docs for the method.
    pub docs: Vec<Doc>,

    /// The request for the method.
    pub request: Request,

    /// The responses for the method.
    pub responses: Vec<Response>,
}

impl Method {
    fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        self.request
            .iter_all_params()
            .chain(self.responses.iter().flat_map(|r| r.iter_all_params()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Documentation
pub struct Doc {
    /// The title of the documentation.
    pub title: Option<String>,

    /// The language of the documentation.
    pub lang: Option<String>,

    /// The content of the documentation.
    pub content: String,

    /// The namespace of the documentation.
    pub xmlns: Option<url::Url>,
}

impl Doc {
    /// Create a new documentation object.
    pub fn new(content: String) -> Self {
        Self {
            content,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A link to another resource.
pub struct Link {
    /// The resource type of the link.
    pub resource_type: Option<ResourceTypeRef>,

    /// Optional token that identifies the relationship of the resource identified by the link to
    /// the resource whose representation the link is embedded in. The value is scoped by the value
    /// of the ancestor representation element's profile attribute.
    pub relation: Option<String>,

    /// An optional token that identifies the relationship of the resource whose representation
    /// the link is embedded in to the resource identified by the link. This is the reverse
    /// relationship to that identified by the rel attribute. The value is scoped by the value
    /// of the ancestor representation element's profile attribute.
    pub reverse_relation: Option<String>,

    /// Optional documentation
    pub doc: Option<Doc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A parameter
pub struct Param {
    /// The style of the parameter.
    pub style: ParamStyle,

    /// The ID of the parameter.
    pub id: Option<Id>,

    /// The name of the parameter.
    pub name: String,

    /// The type of the parameter.
    pub r#type: String,

    /// Path of the parameter.
    pub path: Option<String>,

    /// Whether the parameter is required.
    pub required: bool,

    /// Whether the parameter is repeating.
    pub repeating: bool,

    /// The fixed value of the parameter.
    pub fixed: Option<String>,

    /// The documentation for the parameter.
    pub doc: Option<Doc>,

    /// The links for the parameter.
    pub links: Vec<Link>,

    /// The options for the parameter.
    pub options: Option<Options>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// A representation definition
pub struct RepresentationDef {
    /// The ID of the representation.
    pub id: Option<Id>,

    /// The media type of the representation.
    pub media_type: Option<mime::Mime>,

    /// The element of the representation.
    pub element: Option<String>,

    /// The profile of the representation.
    pub profile: Option<String>,

    /// The documentation for the representation.
    pub docs: Vec<Doc>,

    /// The parameters for the representation.
    pub params: Vec<Param>,
}

impl RepresentationDef {
    fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        self.params.iter()
    }
}

#[derive(Debug, Clone)]
/// A reference to a representation.
pub enum RepresentationRef {
    /// A reference to a representation defined in the same document.
    Id(Id),

    /// A reference to a representation defined in another document.
    Link(Url),
}

impl RepresentationRef {
    /// Return the ID of the representation reference.
    pub fn id(&self) -> Option<&str> {
        match self {
            RepresentationRef::Id(id) => Some(id),
            RepresentationRef::Link(l) => l.fragment(),
        }
    }
}

#[derive(Debug, Clone)]
/// A representation
pub enum Representation {
    /// A reference to a representation defined in the same document.
    Reference(RepresentationRef),

    /// A definition of a representation.
    Definition(RepresentationDef),
}

impl Representation {
    /// Return the content type of this representation.
    pub fn media_type(&self) -> Option<&mime::Mime> {
        match self {
            Representation::Reference(_) => None,
            Representation::Definition(d) => d.media_type.as_ref(),
        }
    }

    /// Return the URL of this representation.
    pub fn url(&self, base_url: &Url) -> Option<Url> {
        match self {
            Representation::Reference(RepresentationRef::Id(id)) => {
                let mut url = base_url.clone();
                url.set_fragment(Some(id));
                Some(url)
            }
            Representation::Reference(RepresentationRef::Link(l)) => Some(l.clone()),
            Representation::Definition(d) => d.url(base_url),
        }
    }

    /// Return the definition of this representation.
    pub fn as_def(&self) -> Option<&RepresentationDef> {
        match self {
            Representation::Reference(_) => None,
            Representation::Definition(d) => Some(d),
        }
    }

    /// Iterate over all parameters defined in this representation.
    pub fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        // TODO: Make this into a proper iterator
        let params = match self {
            Representation::Reference(_) => vec![],
            Representation::Definition(d) => d.iter_all_params().collect::<Vec<_>>(),
        };

        params.into_iter()
    }
}

#[test]
fn test_representation_url() {
    let base_url = Url::parse("http://example.com").unwrap();
    let r = Representation::Reference(RepresentationRef::Id("foo".to_string()));
    assert_eq!(
        r.url(&base_url).unwrap(),
        Url::parse("http://example.com#foo").unwrap()
    );
    let r = Representation::Reference(RepresentationRef::Link(
        Url::parse("http://example.com#foo").unwrap(),
    ));
    assert_eq!(
        r.url(&base_url).unwrap(),
        Url::parse("http://example.com#foo").unwrap()
    );
    let r = Representation::Definition(RepresentationDef {
        id: Some("foo".to_string()),
        ..Default::default()
    });
    assert_eq!(
        r.url(&base_url).unwrap(),
        Url::parse("http://example.com#foo").unwrap()
    );
}

#[test]
fn test_representation_id() {
    let r = Representation::Reference(RepresentationRef::Id("foo".to_string()));
    assert_eq!(r.as_def(), None);
    let r = Representation::Definition(RepresentationDef {
        id: Some("foo".to_string()),
        ..Default::default()
    });
    assert_eq!(r.as_def().unwrap().id, Some("foo".to_string()));
}

impl RepresentationDef {
    /// Fully qualify the URL of this representation.
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

#[derive(Debug, Default, Clone)]
/// A request
pub struct Request {
    /// The docs for the request.
    pub docs: Vec<Doc>,

    /// The parameters for the request.
    pub params: Vec<Param>,

    /// The representations for the request.
    pub representations: Vec<Representation>,
}

impl Request {
    fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        self.params.iter().chain(
            self.representations
                .iter()
                .filter_map(|r| r.as_def().map(|r| r.iter_all_params()))
                .flatten(),
        )
    }
}

#[derive(Debug, Clone, Default)]
/// A response
pub struct Response {
    /// The docs for the response.
    pub docs: Vec<Doc>,

    /// The parameters for the response.
    pub params: Vec<Param>,

    /// The status of the response.
    pub status: Option<i32>,

    /// The representations for the response.
    pub representations: Vec<Representation>,
}

impl Response {
    fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        self.params.iter().chain(
            self.representations
                .iter()
                .filter_map(|r| r.as_def().map(|r| r.iter_all_params()))
                .flatten(),
        )
    }
}

#[derive(Debug)]
/// A resource type
pub struct ResourceType {
    /// The ID of the resource type.
    pub id: Id,

    /// The query type of the resource type.
    pub query_type: mime::Mime,

    /// The methods defined at this level.
    pub methods: Vec<Method>,

    /// The docs for the resource type.
    pub docs: Vec<Doc>,

    /// The subresources of the resource type.
    pub subresources: Vec<Resource>,

    /// The params for the resource type.
    pub params: Vec<Param>,
}

impl ResourceType {
    /// Iterate over all parameters defined in this resource type.
    pub(crate) fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        self.params
            .iter()
            .chain(self.methods.iter().flat_map(|m| m.iter_all_params()))
    }

    /// Returns an iterator over all types referenced by this resource type.
    pub fn iter_referenced_types(&self) -> impl Iterator<Item = String> + '_ {
        self.iter_all_params().map(|p| p.r#type.clone())
    }
}
