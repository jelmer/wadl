use crate::ast::*;

use crate::WADL_MIME_TYPE;

// Convert wadl names (with dashes) to camel-case Rust names
fn camel_case_name(name: &str) -> String {
    let mut it = name.chars().peekable();
    let mut result = String::new();
    // Uppercase the first letter
    if let Some(c) = it.next() {
        result.push_str(&c.to_uppercase().collect::<String>());
    }
    while it.peek().is_some() {
        let c = it.next().unwrap();
        if c == '_' || c == '-' {
            if let Some(next) = it.next() {
                result.push_str(&next.to_uppercase().collect::<String>());
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[test]
fn test_camel_case_name() {
    assert_eq!(camel_case_name("foo-bar"), "FooBar");
    assert_eq!(camel_case_name("foo-bar-baz"), "FooBarBaz");
    assert_eq!(camel_case_name("foo-bar-baz-quux"), "FooBarBazQuux");
    assert_eq!(camel_case_name("_foo-bar"), "_fooBar");
    assert_eq!(camel_case_name("service-root-json"), "ServiceRootJson");
}

fn snake_case_name(name: &str) -> String {
    let mut name = name.to_string();
    name = name.replace('-', "_");
    let mut it = name.chars().peekable();
    let mut result = String::new();
    while it.peek().is_some() {
        let c = it.next().unwrap();
        if c.is_uppercase() {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            result.push_str(&c.to_lowercase().collect::<String>());
        } else {
            result.push(c);
        }
    }
    result
}

#[test]
fn test_snake_case_name() {
    assert_eq!(snake_case_name("FooBar"), "foo_bar");
    assert_eq!(snake_case_name("FooBarBaz"), "foo_bar_baz");
    assert_eq!(snake_case_name("FooBarBazQuux"), "foo_bar_baz_quux");
    assert_eq!(snake_case_name("_FooBar"), "_foo_bar");
}

fn strip_code_examples(input: String) -> String {
    let mut in_example = false;
    input.lines().filter(|line| {
        if !in_example && (line.starts_with("```python") || *line == "```") {
            in_example = true;
            false
        } else if line.starts_with("```") {
            in_example = false;
            false
        } else { !in_example }
    }).collect::<Vec<_>>().join("\n")
}

fn format_doc(input: &Doc, config: &Config) -> String {
    match input.xmlns.as_ref().map(|x| x.as_str()) {
        Some("http://www.w3.org/1999/xhtml") => {
            let mut text = html2md::parse_html(&input.content);
            if config.strip_code_examples {
                text = strip_code_examples(text);
            }
            text
            .lines()
            .collect::<Vec<_>>()
            .join("\n")
        },
        Some(xmlns) => {
            log::warn!("Unknown xmlns: {}", xmlns);
            input.content.lines().collect::<Vec<_>>().join("\n")
        }
        None => input.content.lines().collect::<Vec<_>>().join("\n"),
    }
}

fn generate_doc(input: &Doc, indent: usize, config: &Config) -> Vec<String> {
    let mut lines: Vec<String> = vec![];

    if let Some(title) = input.title.as_ref() {
        lines.extend(vec![format!("/// # {}\n", title), "///\n".to_string()]);
    }

    let text = format_doc(input, config);

    lines.extend(text.lines().map(|line| format!("/// {}\n", line)));
    if indent > 0 {
        lines = lines
            .into_iter()
            .map(|line| format!("{:indent$}{}", "", line.trim_end_matches(' '), indent = indent * 4))
            .collect();
    }
    lines
}

fn generate_representation(input: &RepresentationDef, config: &Config) -> Vec<String> {
    let mut lines = vec![];
    for doc in &input.docs {
        lines.extend(generate_doc(doc, 0, config));
    }

    if input.media_type == Some(mime::APPLICATION_JSON) {
        lines.extend(generate_representation_struct_json(input, config));
    } else {
        panic!("Unknown media type: {:?}", input.media_type);
    }

    let name = input.id.as_ref().unwrap().as_str();
    let name = camel_case_name(name);

    lines.push(format!("impl {} {{\n", name));

    for param in &input.params {
        let field_name = snake_case_name(param.name.as_str());
        let accessor_name = if let Some(rename_fn) = config.param_accessor_rename.as_ref() {
            rename_fn(param.name.as_str())
        } else {
            None
        }
        .unwrap_or_else(|| field_name.to_string());

        // We expect to support multiple types here in the future
        #[allow(clippy::single_match)]
        match &param.r#type {
            TypeRef::ResourceType(r) => {
                if let Some(id) = r.id() {
                    for doc in &param.doc {
                        lines.extend(generate_doc(doc, 1, config));
                    }
                    let field_type = camel_case_name(id);
                    let mut ret_type = format!("Box<dyn {}>", field_type);
                    let map_fn = if let Some((map_type, map_fn)) = config.map_type_for_accessor.as_ref().and_then(|x| x(field_type.as_str())) {
                        ret_type = map_type;
                        Some(map_fn)
                    } else {
                        None
                    };
                    if !param.required {
                        ret_type = format!("Option<{}>", ret_type);
                    }
                    let visibility = config.accessor_visibility.as_ref().and_then(|x| x(accessor_name.as_str(), field_type.as_str())).unwrap_or_else(|| "pub".to_string());
                    lines.push(format!(
                        "    {}fn {}(&self) -> {} {{\n",
                        if visibility.is_empty() {
                            "".to_string()
                        } else {
                            format!("{} ", visibility)
                        }, accessor_name, ret_type
                    ));
                    lines.push("        struct MyResource(url::Url);\n".to_string());
                    lines.push("        impl Resource for MyResource { fn url(&self) -> url::Url { self.0.clone() } }\n".to_string());
                    lines.push(format!("        impl {} for MyResource {{}}\n", field_type));
                    if param.required {
                        if let Some(map_fn) = map_fn {
                            lines.push(format!(
                                "        {}(Box::new(MyResource(self.{}.clone()) as Box<dyn {}>))\n",
                                map_fn, field_name, field_type
                            ));
                        } else {
                            lines.push(format!(
                                "        Box::new(MyResource(self.{}.clone()))\n",
                                field_name
                            ));
                        }
                    } else {
                        lines.push(format!(
                        "        self.{}.as_ref().map(|x| Box::new(MyResource(x.clone())) as Box<dyn {}>){}\n",
                        field_name, field_type, if let Some(map_fn) = map_fn { format!(".map({})", map_fn) } else { "".to_string() }
                    ));
                    }
                    lines.push("    }\n".to_string());
                    lines.push("\n".to_string());

                    if let Some(extend_accessor) = config.extend_accessor.as_ref() {
                        lines.extend(extend_accessor(accessor_name.as_str(), ret_type.as_str()));
                    }
                }
            }
            _ => {}
        }
    }

    lines.push("}\n".to_string());
    lines.push("\n".to_string());

    if let Some(generate) = config.generate_representation_traits.as_ref() {
        lines.extend(generate(name.as_str(), input, config).unwrap_or(vec![]));
    }

    lines
}

fn param_rust_type(param: &Param, config: &Config) -> (String, Vec<String>) {
    assert!(param.id.is_none());
    assert!(param.fixed.is_none());

    let (mut param_type, annotations) = match &param.r#type {
        TypeRef::Simple(name) => match name.as_str() {
            "xsd:date" => ("chrono::NaiveDate".to_string(), vec![]),
            "xsd:dateTime" => ("chrono::DateTime<chrono::Utc>".to_string(), vec![]),
            "xsd:time" => ("(chrono::Time".to_string(), vec![]),
            "string" => ("String".to_string(), vec![]),
            "binary" => ("Vec<u8>".to_string(), vec![]),
            u => panic!("Unknown type: {}", u),
        },
        // TODO: Return the actual type, not a URL
        TypeRef::ResourceType(_) => ("url::Url".to_string(), vec![]),
        TypeRef::Options(_options) => {
            // TODO: define an enum for this
            ("String".to_string(), vec![])
        }
        TypeRef::NoType => {
            let tn = if let Some(guess_name) = config.guess_type_name.as_ref() {
                guess_name(param.name.as_str())
            } else {
                None
            };

            if let Some(tn) = tn {
                (tn, vec![])
            } else {
                log::warn!("No type for parameter: {}", param.name);
                ("serde_json::Value".to_string(), vec![])
            }
        }
    };

    if param.repeating {
        param_type = format!("Vec<{}>", param_type);
    }

    if !param.required {
        param_type = format!("Option<{}>", param_type);
    }

    (param_type, annotations)
}

fn readonly_rust_type(name: &str) -> String {
    match name {
        "String" => "&str".to_string(),
        x if x.starts_with("Option<") && x.ends_with('>') => {
            format!("Option<&{}>", x[7..x.len() - 1].trim())
        }
        x if x.starts_with("Vec<") && x.ends_with('>') => {
            format!("&[{}]", x[4..x.len() - 1].trim())
        }
        x => format!("&{}", x),
    }
}

fn generate_representation_struct_json(input: &RepresentationDef, config: &Config) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let name = input.id.as_ref().unwrap().as_str();
    let name = camel_case_name(name);

    lines.push(format!("/// Representation of the `{}` resource\n", input.id.as_ref().unwrap()));

    lines.push(
        "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n".to_string(),
    );
    lines.push(format!("pub struct {} {{\n", name));

    for param in &input.params {
        let mut param_name = snake_case_name(param.name.as_str());

        if ["type", "move"].contains(&param_name.as_str()) {
            param_name = format!("r#{}", param_name);
        }

        let (param_type, annotations) = param_rust_type(param, config);
        let comment = match &param.r#type {
            TypeRef::Simple(name) => format!("was: {}", name),
            TypeRef::ResourceType(r) => match r {
                ResourceTypeRef::Id(id) => format!("resource type id: {}", id),
                ResourceTypeRef::Link(href) => format!("resource type link: {}", href),
                ResourceTypeRef::Empty => "was: empty link".to_string(),
            },
            TypeRef::Options(options) => format!("options: {:?}", options),
            TypeRef::NoType => "no type for parameter in WADL".to_string(),
        };

        let is_pub = !matches!(&param.r#type, TypeRef::ResourceType(_));

        lines.push(format!("    // {}\n", comment));
        for ann in annotations {
            lines.push(format!("    {}\n", ann));
        }
        lines.push(format!(
            "    {}{}: {},\n",
            if is_pub { "pub " } else { "" },
            param_name,
            param_type
        ));
    }

    lines.push("}\n".to_string());
    lines.push("\n".to_string());

    lines
}

fn supported_representation_def(d: &RepresentationDef) -> bool {
    d.media_type != Some(WADL_MIME_TYPE.parse().unwrap())
        && d.media_type != Some("application/xhtml+xml".parse().unwrap())
}

pub fn rust_type_for_response(input: &Response, name: &str) -> String {
    let representations = input
        .representations
        .iter()
        .filter(|r| match r {
            Representation::Definition(ref d) => supported_representation_def(d),
            _ => true,
        })
        .collect::<Vec<_>>();
    if representations.len() == 1 {
        assert!(input.params.is_empty());
        match representations[0] {
            Representation::Reference(ref r) => {
                let id = r.id().unwrap().to_string();
                camel_case_name(id.as_str())
            }
            Representation::Definition(ref d) => {
                assert!(d.params.iter().all(|p| p.style == ParamStyle::Header));

                let mut ret = Vec::new();
                for param in &input.params {
                    let (param_type, _annotations) = param_rust_type(param, &Config::default());
                    ret.push(param_type);
                }
                if ret.len() == 1 {
                    ret[0].clone()
                } else {
                    format!("({})", ret.join(", "))
                }
            }
        }
    } else if representations.is_empty() {
        let mut ret = Vec::new();
        for param in &input.params {
            let (param_type, _annotations) = param_rust_type(param, &Config::default());
            ret.push(param_type);
        }
        if ret.len() == 1 {
            ret[0].clone()
        } else {
            format!("({})", ret.join(", "))
        }
    } else {
        todo!(
            "multiple representations for response: {}: {:?}",
            name,
            representations
        );
    }
}

pub fn format_arg_doc(name: &str, doc: Option<&crate::ast::Doc>, config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(doc) = doc.as_ref() {
        let doc = format_doc(doc, config);
        let mut doc_lines = doc
            .trim_start_matches('\n')
            .split('\n')
            .collect::<Vec<_>>()
            .into_iter();
        lines.push(format!(
            "    /// * `{}`: {}\n",
            name,
            doc_lines.next().unwrap().trim_end_matches(' ')
        ));
        for doc_line in doc_lines {
            if doc_line.is_empty() {
                lines.push("    ///\n".to_string());
            } else {
                lines.push(format!("    ///     {}\n", doc_line.trim_end_matches(' ')));
            }
        }
    } else {
        lines.push(format!("    /// * `{}`\n", name));
    }

    lines
}


pub fn generate_method(input: &Method, parent_id: &str, config: &Config) -> Vec<String> {
    let mut lines = vec![];

    let name = input.id.as_str();
    let name = name
        .strip_prefix(format!("{}-", parent_id).as_str())
        .unwrap_or(name);
    let name = snake_case_name(name);

    let mut line = format!("    fn {}(&self, client: &dyn wadl::Client", name);

    let mut params = input.request.params.iter().collect::<Vec<_>>();

    params.extend(
        input
            .request
            .representations
            .iter()
            .filter_map(|r| match r {
                Representation::Definition(d) => Some(&d.params),
                Representation::Reference(_) => None,
            })
            .flatten(),
    );

    for doc in &input.docs {
        lines.extend(generate_doc(doc, 1, config));
    }

    if !params.is_empty() {
        lines.push("    /// # Arguments\n".to_string());
    }

    for representation in &input.request.representations {
        match representation {
            Representation::Definition(_) => {},
            Representation::Reference(r) => {
                let id = camel_case_name(r.id().unwrap());
                line.push_str(format!(", representation: &{}", id).as_str());
            }
        }
    }

    for param in &params {
        if param.fixed.is_some() {
            continue;
        }
        let (param_type, _annotations) = param_rust_type(param, config);
        let param_type = readonly_rust_type(param_type.as_str());
        let mut param_name = param.name.clone();
        if ["type", "move"].contains(&param_name.as_str()) {
            param_name = format!("r#{}", param_name);
        }

        line.push_str(format!(", {}: {}", param_name, param_type).as_str());

        lines.extend(format_arg_doc(param_name.as_str(), param.doc.as_ref(), config));
    }
    line.push_str(") -> Result<");
    let map_fn = if input.responses.is_empty() {
        line.push_str("()");
        None
    } else {
        assert_eq!(1, input.responses.len(), "expected 1 response for {}", name);
        let mut return_type = rust_type_for_response(&input.responses[0], input.id.as_str());
        let map_fn = if let Some((map_type, map_fn)) = config.map_type_for_response.as_ref().and_then(|r| r(&return_type)) {
            return_type = map_type;
            Some(map_fn)
        } else {
            None
        };
        line.push_str(return_type.as_str());
        map_fn
    };

    line.push_str(", Error> {\n");
    lines.push(line);

    assert!(input
        .request
        .params
        .iter()
        .all(|p| [ParamStyle::Header, ParamStyle::Query].contains(&p.style)));

    lines.push("        let mut url_ = self.url();\n".to_string());
    for param in params.iter().filter(|p| p.style == ParamStyle::Query) {
        if let Some(fixed) = param.fixed.as_ref() {
            assert!(!param.repeating);
            lines.push(format!(
                "        url_.query_pairs_mut().append_pair(\"{}\", \"{}\");\n",
                param.name, fixed
            ));
        } else {
            let param_name = param.name.as_str();
            let mut param_name = snake_case_name(param_name);
            if ["type", "move"].contains(&param_name.as_str()) {
                param_name = format!("r#{}", param_name);
            }

            let (param_type, _annotations) = param_rust_type(param, config);
            let value = format!("&{}.to_string()", param_name);

            let mut indent = 0;

            let needs_iter = param.repeating
                || param_type.starts_with("Vec<")
                || param_type.starts_with("Option<Vec<");

            if param_type.starts_with("Option<") {
                lines.push(format!(
                    "        if let Some({}) = {} {{\n",
                    param_name, param_name
                ));
                indent += 4;
            }
            if needs_iter {
                lines.push(format!(
                    "{:indent$}        for {} in {} {{\n",
                    "", param_name, param_name
                ));
                indent += 4;
            }
            lines.push(format!(
                "{:indent$}        url_.query_pairs_mut().append_pair(\"{}\", {});\n",
                "",
                param.name,
                value,
                indent = indent
            ));
            while indent > 0 {
                lines.push(format!("{:indent$}    }}\n", "", indent = indent));
                indent -= 4;
            }
        }
    }

    lines.push("\n".to_string());

    let method = input.name.as_str();
    lines.push(format!(
        "        let mut req = reqwest::blocking::Request::new(reqwest::Method::{}, url_);\n",
        method
    ));

    for representation in &input.request.representations {
        match representation {
            Representation::Definition(_) => { }
            Representation::Reference(_) => {
                lines.push("        let body = serde_json::to_string(&representation)?;\n".to_string());
                // TODO(jelmer): Support non-JSON representations
                lines.push("        req.headers_mut().insert(reqwest::header::CONTENT_TYPE, \"application/json\".parse().unwrap());\n".to_string());
                lines.push("        req.headers_mut().insert(reqwest::header::CONTENT_LENGTH, body.len().to_string().parse().unwrap());\n".to_string());
                lines.push("        *req.body_mut() = Some(reqwest::blocking::Body::from(body));\n".to_string());
            }
        }
    }

    let mime_types = input
        .responses
        .iter()
        .flat_map(|x| {
            x.representations.iter().filter_map(|x| match x {
                Representation::Definition(ref d) if supported_representation_def(d) => {
                    d.media_type.clone()
                }
                Representation::Reference(_) => {
                    // TODO: Look up media type of reference
                    Some(mime::APPLICATION_JSON)
                }
                _ => None,
            })
        })
        .collect::<Vec<_>>();

    if !mime_types.is_empty() {
        lines.push(format!(
            "        req.headers_mut().insert(reqwest::header::ACCEPT, \"{}\".parse().unwrap());\n",
            mime_types
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    for param in params.iter().filter(|p| p.style == ParamStyle::Header) {
        let value = if let Some(fixed) = param.fixed.as_ref() {
            format!("\"{}\"", fixed)
        } else {
            let param_name = param.name.as_str();
            let mut param_name = snake_case_name(param_name);
            if ["type", "move"].contains(&param_name.as_str()) {
                param_name = format!("r#{}", param_name);
            }

            format!("&{}.to_string()", param_name)
        };

        lines.push(format!(
            "        req.headers_mut().insert(\"{}\", {});\n",
            param.name, value
        ));
    }

    lines.push("\n".to_string());
    lines.push("        let resp = client.execute(req)?.error_for_status()?;\n".to_string());
    lines.push("        let text = resp.text()?;\n".to_string());
    lines.push(format!("        serde_json::from_str(&text).map_err(|e| Error::Json(e)){}\n", if let Some(map_fn) = map_fn {
        format!(".map({})", map_fn)
    } else {
        "".to_string()
    }));
    lines.push("    }\n".to_string());
    lines.push("\n".to_string());

    lines
}

fn generate_resource_type(input: &ResourceType, config: &Config) -> Vec<String> {
    let mut lines = vec![];

    for doc in &input.docs {
        lines.extend(generate_doc(doc, 0, config));
    }

    let name = input.id.as_str();
    let name = camel_case_name(name);

    let visibility = config.resource_type_visibility.as_ref().and_then(|x| x(name.as_str())).unwrap_or("pub".to_string());

    lines.push(format!("{}trait {} : Resource {{\n", if visibility.is_empty() {
        "".to_string()
    } else {
        format!("{} ", visibility)
    }, name));

    for method in &input.methods {
        lines.extend(generate_method(method, input.id.as_str(), config));
    }

    lines.push("}\n".to_string());
    lines.push("\n".to_string());
    lines
}

#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct Config {
    /// Based on the name of a parameter, determine the rust type
    pub guess_type_name: Option<Box<dyn Fn(&str) -> Option<String>>>,

    /// Support renaming param accessor functions
    pub param_accessor_rename: Option<Box<dyn Fn(&str) -> Option<String>>>,

    /// Whether to strip code examples from the docstrings
    ///
    /// This is useful if the code examples are not valid rust code.
    pub strip_code_examples: bool,

    /// Generate custom trait implementations for representations
    pub generate_representation_traits: Option<Box<dyn Fn(&str, &RepresentationDef, &Config) -> Option<Vec<String>>>>,

    /// Return the visibility of a representation accessor
    pub accessor_visibility: Option<Box<dyn Fn(&str, &str) -> Option<String>>>,

    /// Return the visibility of a resource type
    pub resource_type_visibility: Option<Box<dyn Fn(&str) -> Option<String>>>,

    /// Map a method response type to a different type and a function to map the response
    pub map_type_for_response: Option<Box<dyn Fn(&str) -> Option<(String, String)>>>,

    /// Map an accessor function name to a different type
    pub map_type_for_accessor: Option<Box<dyn Fn(&str) -> Option<(String, String)>>>,

    /// Extend the generated accessor
    pub extend_accessor: Option<Box<dyn Fn(&'_ str, &'_ str ) -> Vec<String>>>,
}

pub fn generate(app: &Application, config: &Config) -> String {
    let mut lines = vec![];

    for doc in &app.docs {
        lines.extend(generate_doc(doc, 0, config));
    }

    for representation in &app.representations {
        lines.extend(generate_representation(representation, config));
    }

    for resource_type in &app.resource_types {
        lines.extend(generate_resource_type(resource_type, config));
    }

    lines.concat()
}
