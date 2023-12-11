use std::collections::HashMap;
use url::Url;

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

    pub representations: Vec<RepresentationDef>,
}

impl Application {
    pub fn get_resource_type_by_id(&self, id: &str) -> Option<&ResourceType> {
        self.resource_types.iter().find(|rt| id == rt.id.as_str())
    }

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
    pub fn iter_referenced_types(&self) -> impl Iterator<Item = TypeRef> + '_ {
        self.iter_resources()
            .flat_map(|(_u, r)| r.iter_referenced_types())
            .chain(
                self.resource_types
                    .iter()
                    .flat_map(|rt| rt.iter_referenced_types()),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceTypeRef {
    Id(Id),
    Link(Url),
    Empty,
}

impl std::str::FromStr for ResourceTypeRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix('#') {
            Ok(ResourceTypeRef::Id(s.to_string()))
        } else {
            Ok(ResourceTypeRef::Link(
                s.parse().map_err(|e| format!("{}", e))?,
            ))
        }
    }
}

impl ResourceTypeRef {
    pub fn id(&self) -> Option<&str> {
        match self {
            ResourceTypeRef::Id(id) => Some(id),
            ResourceTypeRef::Link(l) => l.fragment(),
            ResourceTypeRef::Empty => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Simple(String),
    ResourceType(ResourceTypeRef),
    NoType,
    Options(HashMap<String, Option<mime::Mime>>),
}

impl std::str::FromStr for TypeRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix('#') {
            Ok(TypeRef::ResourceType(ResourceTypeRef::Id(s.to_string())))
        } else {
            Ok(TypeRef::Simple(s.to_string()))
        }
    }
}

#[derive(Debug, Clone)]
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
        self.params
            .iter()
            .chain(self.methods.iter().flat_map(|m| m.request.params.iter()))
    }

    /// Iterate over all types referenced by this resource.
    pub fn iter_referenced_types(&self) -> impl Iterator<Item = TypeRef> + '_ {
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
pub struct Method {
    pub id: Id,
    pub name: String,
    pub docs: Vec<Doc>,
    pub request: Request,
    pub responses: Vec<Response>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    pub fn new(content: String) -> Self {
        Self {
            content,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
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

    pub doc: Option<Doc>,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub style: ParamStyle,
    pub id: Option<Id>,
    pub name: String,
    pub r#type: TypeRef,
    pub path: Option<String>,
    pub required: bool,
    pub repeating: bool,
    pub fixed: Option<String>,
    pub doc: Option<Doc>,
    pub links: Vec<Link>
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepresentationDef {
    pub id: Option<Id>,
    pub media_type: Option<mime::Mime>,
    pub element: Option<String>,
    pub profile: Option<String>,
    pub docs: Vec<Doc>,
    pub params: Vec<Param>,
}

#[derive(Debug, Clone)]
pub enum RepresentationRef {
    /// A reference to a representation defined in the same document.
    Id(Id),
    Link(Url),
}

impl RepresentationRef {
    pub fn id(&self) -> Option<&str> {
        match self {
            RepresentationRef::Id(id) => Some(id),
            RepresentationRef::Link(l) => l.fragment(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Representation {
    Reference(RepresentationRef),
    Definition(RepresentationDef),
}

impl Representation {
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

    pub fn as_def(&self) -> Option<&RepresentationDef> {
        match self {
            Representation::Reference(_) => None,
            Representation::Definition(d) => Some(d),
        }
    }
}

#[test]
fn test_representation_url() {
    let base_url = Url::parse("http://example.com").unwrap();
    let r = Representation::Reference(RepresentationRef::Id("foo".to_string()));
    assert_eq!(r.url(&base_url).unwrap(), Url::parse("http://example.com#foo").unwrap());
    let r = Representation::Reference(RepresentationRef::Link(Url::parse("http://example.com#foo").unwrap()));
    assert_eq!(r.url(&base_url).unwrap(), Url::parse("http://example.com#foo").unwrap());
    let r = Representation::Definition(RepresentationDef {
        id: Some("foo".to_string()),
        ..Default::default()
    });
    assert_eq!(r.url(&base_url).unwrap(), Url::parse("http://example.com#foo").unwrap());
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
pub struct Request {
    pub docs: Vec<Doc>,
    pub params: Vec<Param>,
    pub representations: Vec<Representation>,
}

#[derive(Debug, Clone, Default)]
pub struct Response {
    pub docs: Vec<Doc>,
    pub params: Vec<Param>,
    pub status: Option<i32>,
    pub representations: Vec<Representation>,
}

#[derive(Debug)]
pub struct ResourceType {
    pub id: Id,
    pub query_type: mime::Mime,
    pub methods: Vec<Method>,
    pub docs: Vec<Doc>,
    pub subresources: Vec<Resource>,
    pub params: Vec<Param>,
}

impl ResourceType {
    pub(crate) fn iter_all_params(&self) -> impl Iterator<Item = &Param> {
        self.params
            .iter()
            .chain(self.methods.iter().flat_map(|m| m.request.params.iter()))
    }

    /// Returns an iterator over all types referenced by this resource type.
    pub fn iter_referenced_types(&self) -> impl Iterator<Item = TypeRef> + '_ {
        self.iter_all_params().map(|p| p.r#type.clone())
    }
}
