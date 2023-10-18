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

    pub representations: Vec<Representation>,
}

impl Application {
    pub fn get_resource_type_by_id(&self, id: &str) -> Option<&ResourceType> {
        self.resource_types
            .iter()
            .find(|rt| rt.id.as_ref().map(|i| i == id).unwrap_or(false))
    }

    pub fn get_resource_type_by_href(&self, href: &Url) -> Option<&ResourceType> {
        // TODO(jelmer): Check that href matches us?
        if let Some(fragment) = href.fragment() {
            self.get_resource_type_by_id(fragment)
        } else {
            None
        }
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

#[derive(Debug)]
pub enum TypeRef {
    Simple(String),
    ResourceTypeId(Id),
    ResourceTypeLink(Url),
    EmptyLink,
    NoType,
    Options(HashMap<String, Option<String>>),
}

impl std::str::FromStr for TypeRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix('#') {
            Ok(TypeRef::ResourceTypeId(s.to_string()))
        } else {
            Ok(TypeRef::Simple(s.to_string()))
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

impl Resource {
    pub fn url(&self, base_url: Option<&Url>) -> Url {
        if let Some(base_url) = base_url {
            base_url.join(self.path.as_ref().unwrap()).unwrap()
        } else {
            Url::parse(self.path.as_ref().unwrap()).unwrap()
        }
    }
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

    /// The namespace of the documentation.
    pub xmlns: Option<url::Url>,
}

#[derive(Debug)]
pub struct Param {
    pub style: ParamStyle,
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
