//! Generate Rust code from WADL files

use crate::ast::*;
use std::collections::HashMap;

/// MIME type for XHTML
pub const XHTML_MIME_TYPE: &str = "application/xhtml+xml";

#[allow(missing_docs)]
pub enum ParamContainer<'a> {
    Request(&'a Method, &'a Request),
    Response(&'a Method, &'a Response),
    Representation(&'a RepresentationDef),
}

/// Convert wadl names (with dashes) to camel-case Rust names
pub fn camel_case_name(name: &str) -> String {
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

/// Convert wadl names (with dashes) to snake-case Rust names
pub fn snake_case_name(name: &str) -> String {
    let mut name = name.to_string();
    name = name.replace('-', "_");
    let it = name.chars().peekable();
    let mut result = String::new();
    let mut started = false;
    for c in it {
        if c.is_uppercase() {
            if !result.is_empty() && !started && !result.ends_with('_') {
                result.push('_');
                started = true;
            }
            result.push_str(c.to_lowercase().collect::<String>().as_str());
        } else {
            result.push(c);
            started = false;
        }
    }
    result
}

fn strip_code_examples(input: String) -> String {
    let mut in_example = false;
    input
        .lines()
        .filter(|line| {
            if !in_example && (line.starts_with("```python") || *line == "```") {
                in_example = true;
                false
            } else if line.starts_with("```") {
                in_example = false;
                false
            } else {
                !in_example
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format the given `Doc` object into a string.
///
/// # Arguments
/// * `input` - The `Doc` object to format.
/// * `config` - The configuration to use.
///
/// # Returns
/// The formatted string.
fn format_doc(input: &Doc, config: &Config) -> String {
    match input.xmlns.as_ref().map(|x| x.as_str()) {
        Some("http://www.w3.org/1999/xhtml") => {
            let mut text = html2md::parse_html(&input.content);
            if config.strip_code_examples {
                text = strip_code_examples(text);
            }
            text.lines().collect::<Vec<_>>().join("\n")
        }
        Some(xmlns) => {
            log::warn!("Unknown xmlns: {}", xmlns);
            input.content.lines().collect::<Vec<_>>().join("\n")
        }
        None => input.content.lines().collect::<Vec<_>>().join("\n"),
    }
}

/// Generate a docstring from the given `Doc` object.
///
/// # Arguments
/// * `input` - The `Doc` object to generate the docstring from.
/// * `indent` - The indentation level to use.
/// * `config` - The configuration to use.
///
/// # Returns
/// A vector of strings, each representing a line of the docstring.
pub fn generate_doc(input: &Doc, indent: usize, config: &Config) -> Vec<String> {
    let mut lines: Vec<String> = vec![];

    if let Some(title) = input.title.as_ref() {
        lines.extend(vec![format!("/// # {}\n", title), "///\n".to_string()]);
    }

    let mut text = format_doc(input, config);

    if let Some(reformat_docstring) = config.reformat_docstring.as_ref() {
        text = reformat_docstring(&text);
    }

    lines.extend(
        text.lines()
            .map(|line| format!("///{}{}\n", if line.is_empty() { "" } else { " " }, line)),
    );
    lines
        .into_iter()
        .map(|line| format!("{:indent$}{}", "", line, indent = indent * 4))
        .collect()
}

fn generate_resource_type_ref_accessors(
    field_name: &str,
    input: &ResourceTypeRef,
    param: &Param,
    config: &Config,
) -> Vec<String> {
    let mut lines = vec![];
    if let Some(id) = input.id() {
        let deprecated = config
            .deprecated_param
            .as_ref()
            .map(|x| x(param))
            .unwrap_or(false);
        if let Some(doc) = param.doc.as_ref() {
            lines.extend(generate_doc(doc, 1, config));
        }
        let field_type = camel_case_name(id);
        let mut ret_type = field_type.to_string();
        let map_fn = if let Some((map_type, map_fn)) = config
            .map_type_for_accessor
            .as_ref()
            .and_then(|x| x(field_type.as_str()))
        {
            ret_type = map_type;
            Some(map_fn)
        } else {
            None
        };
        if config.nillable(param) {
            ret_type = format!("Option<{}>", ret_type);
        }
        let accessor_name = if let Some(rename_fn) = config.param_accessor_rename.as_ref() {
            rename_fn(param.name.as_str(), ret_type.as_str())
        } else {
            None
        }
        .unwrap_or_else(|| field_name.to_string());

        let visibility = config
            .accessor_visibility
            .as_ref()
            .and_then(|x| x(accessor_name.as_str(), field_type.as_str()))
            .unwrap_or_else(|| "pub".to_string());
        if deprecated {
            lines.push("    #[deprecated]".to_string());
        }
        lines.push(format!(
            "    {}fn {}(&self) -> {} {{\n",
            if visibility.is_empty() {
                "".to_string()
            } else {
                format!("{} ", visibility)
            },
            accessor_name,
            ret_type
        ));
        if !config.nillable(param) {
            if let Some(map_fn) = map_fn {
                lines.push(format!(
                    "        {}({}(self.{}.clone())\n",
                    map_fn, field_type, field_name
                ));
            } else {
                lines.push(format!(
                    "        {}(self.{}.clone())\n",
                    field_type, field_name
                ));
            }
        } else {
            lines.push(format!(
                "        self.{}.as_ref().map(|x| {}(x.clone())){}\n",
                field_name,
                field_type,
                if let Some(map_fn) = map_fn {
                    format!(".map({})", map_fn)
                } else {
                    "".to_string()
                }
            ));
        }
        lines.push("    }\n".to_string());
        lines.push("\n".to_string());

        if deprecated {
            lines.push("    #[deprecated]".to_string());
        }

        lines.push(format!(
            "    {}fn set_{}(&mut self, value: {}) {{\n",
            if visibility.is_empty() {
                "".to_string()
            } else {
                format!("{} ", visibility)
            },
            accessor_name,
            ret_type
        ));

        if !config.nillable(param) {
            lines.push(format!(
                "        self.{} = value.url().clone();\n",
                field_name
            ));
        } else {
            lines.push(format!(
                "        self.{} = value.map(|x| x.url().clone());\n",
                field_name
            ));
        }
        lines.push("    }\n".to_string());

        if let Some(extend_accessor) = config.extend_accessor.as_ref() {
            lines.extend(extend_accessor(
                param,
                accessor_name.as_str(),
                ret_type.as_str(),
                config,
            ));
        }
    }
    lines
}

fn generate_representation(
    input: &RepresentationDef,
    config: &Config,
    options_names: &HashMap<Options, String>,
) -> Vec<String> {
    let mut lines = vec![];
    if input.media_type == Some(mime::APPLICATION_JSON) {
        lines.extend(generate_representation_struct_json(
            input,
            config,
            options_names,
        ));
    } else {
        panic!("Unknown media type: {:?}", input.media_type);
    }

    let name = input.id.as_ref().unwrap().as_str();
    let name = camel_case_name(name);

    lines.push(format!("impl {} {{\n", name));

    for param in &input.params {
        let field_name = snake_case_name(param.name.as_str());
        // We expect to support multiple types here in the future
        for link in &param.links {
            if let Some(r) = link.resource_type.as_ref() {
                lines.extend(generate_resource_type_ref_accessors(
                    &field_name,
                    r,
                    param,
                    config,
                ));
            }
        }
    }

    lines.push("}\n".to_string());
    lines.push("\n".to_string());

    if let Some(generate) = config.generate_representation_traits.as_ref() {
        lines.extend(generate(input, name.as_str(), input, config).unwrap_or(vec![]));
    }

    lines
}

/// Generate the Rust type for a representation
fn resource_type_rust_type(r: &ResourceTypeRef) -> String {
    if let Some(id) = r.id() {
        camel_case_name(id)
    } else {
        "url::Url".to_string()
    }
}

fn simple_type_rust_type(
    container: &ParamContainer,
    type_name: &str,
    param: &Param,
    config: &Config,
) -> (String, Vec<String>) {
    let tn = if let Some(override_name) = config.override_type_name.as_ref() {
        override_name(container, type_name, param.name.as_str(), config)
    } else {
        None
    };

    if let Some(tn) = tn {
        return (tn, vec![]);
    }

    match type_name.split_once(':').map_or(type_name, |(_, n)| n) {
        "date" => ("chrono::NaiveDate".to_string(), vec![]),
        "dateTime" => ("chrono::DateTime<chrono::Utc>".to_string(), vec![]),
        "time" => ("(chrono::Time".to_string(), vec![]),
        "int" => ("i32".to_string(), vec![]),
        "string" => ("String".to_string(), vec![]),
        "binary" => ("Vec<u8>".to_string(), vec![]),
        "boolean" => ("bool".to_string(), vec![]),
        u => panic!("Unknown type: {}", u),
    }
}

fn param_rust_type(
    container: &ParamContainer,
    param: &Param,
    config: &Config,
    resource_type_rust_type: impl Fn(&ResourceTypeRef) -> String,
    options_names: &HashMap<Options, String>,
) -> (String, Vec<String>) {
    let (mut param_type, annotations) = if !param.links.is_empty() {
        if let Some(rt) = param.links[0].resource_type.as_ref() {
            let name = resource_type_rust_type(rt);

            if let Some(override_type_name) = config
                .override_type_name
                .as_ref()
                .and_then(|x| x(container, name.as_str(), param.name.as_str(), config))
            {
                (override_type_name, vec![])
            } else {
                (name, vec![])
            }
        } else {
            ("url::Url".to_string(), vec![])
        }
    } else if let Some(os) = param.options.as_ref() {
        let options_name = options_names.get(os).unwrap_or_else(|| {
            panic!("Unknown options {:?} for {}", os, param.name);
        });
        (options_name.clone(), vec![])
    } else {
        simple_type_rust_type(container, param.r#type.as_str(), param, config)
    };

    if param.repeating {
        param_type = format!("Vec<{}>", param_type);
    }

    if config.nillable(param) {
        param_type = format!("Option<{}>", param_type);
    }

    (param_type, annotations)
}

fn readonly_rust_type(name: &str) -> String {
    if name.starts_with("Option<") && name.ends_with('>') {
        return format!(
            "Option<{}>",
            readonly_rust_type(name[7..name.len() - 1].trim())
        );
    }
    match name {
        "String" => "&str".to_string(),
        x if x.starts_with("Vec<") && x.ends_with('>') => {
            format!("&[{}]", x[4..x.len() - 1].trim())
        }
        x if x.starts_with('*') => x[1..].to_string(),
        x => format!("&{}", x),
    }
}

fn representation_rust_type(r: &RepresentationRef) -> String {
    if let Some(id) = r.id() {
        camel_case_name(id)
    } else {
        "serde_json::Value".to_string()
    }
}

fn escape_rust_reserved(name: &str) -> &str {
    match name {
        "type" => "r#type",
        "match" => "r#match",
        "move" => "r#move",
        "use" => "r#use",
        "loop" => "r#loop",
        "continue" => "r#continue",
        "break" => "r#break",
        "fn" => "r#fn",
        "struct" => "r#struct",
        "enum" => "r#enum",
        "trait" => "r#trait",
        "impl" => "r#impl",
        "pub" => "r#pub",
        "as" => "r#as",
        "const" => "r#const",
        "let" => "r#let",
        name => name,
    }
}

fn generate_representation_struct_json(
    input: &RepresentationDef,
    config: &Config,
    options_names: &HashMap<Options, String>,
) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let name = input.id.as_ref().unwrap().as_str();
    let name = camel_case_name(name);

    let container = ParamContainer::Representation(input);

    for doc in &input.docs {
        lines.extend(generate_doc(doc, 0, config));
    }

    if input.docs.is_empty() {
        lines.push(format!(
            "/// Representation of the `{}` resource\n",
            input.id.as_ref().unwrap()
        ));
    }

    let derive_default = input.params.iter().all(|x| config.nillable(x));

    lines.push(
        "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n".to_string(),
    );

    let visibility = config
        .representation_visibility
        .as_ref()
        .and_then(|x| x(name.as_str()))
        .unwrap_or_else(|| "pub".to_string());

    lines.push(format!(
        "{}struct {} {{\n",
        if visibility.is_empty() {
            "".to_string()
        } else {
            format!("{} ", visibility)
        },
        name
    ));

    for param in &input.params {
        let param_name = snake_case_name(param.name.as_str());

        let param_name = escape_rust_reserved(param_name.as_str());

        let (param_type, annotations) = param_rust_type(
            &container,
            param,
            config,
            |_x| "url::Url".to_string(),
            options_names,
        );

        // We provide accessors for resource types
        let is_pub = true;

        lines.push(format!("    // was: {}\n", param.r#type));
        if let Some(doc) = param.doc.as_ref() {
            lines.extend(generate_doc(doc, 1, config));
        }

        for ann in annotations {
            lines.push(format!("    {}\n", ann));
        }
        lines.push(format!(
            "    {}{}: {},\n",
            if is_pub { "pub " } else { "" },
            param_name,
            param_type
        ));
        lines.push("\n".to_string());
    }

    lines.push("}\n".to_string());

    if derive_default {
        lines.push(format!("impl Default for {} {{\n", name));
        lines.push("    fn default() -> Self {\n".to_string());
        lines.push("        Self {\n".to_string());
        for param in &input.params {
            let param_name = snake_case_name(param.name.as_str());

            let param_name = escape_rust_reserved(param_name.as_str());

            lines.push(format!("            {}: Default::default(),\n", param_name));
        }

        lines.push("        }\n".to_string());
        lines.push("    }\n".to_string());
        lines.push("}\n".to_string());
        lines.push("\n".to_string());
    }

    lines.push("\n".to_string());

    lines
}

fn supported_representation_def(_d: &RepresentationDef) -> bool {
    false
}

/// Generate the Rust type for a representation
///
/// # Arguments
/// * `input` - The representation to generate the Rust type for
/// * `name` - The name of the representation
///
/// # Returns
///
/// The Rust type for the representation
fn rust_type_for_response(
    method: &Method,
    input: &Response,
    name: &str,
    options_names: &HashMap<Options, String>,
) -> String {
    let container = ParamContainer::Response(method, input);
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
                    let (param_type, _annotations) = param_rust_type(
                        &container,
                        param,
                        &Config::default(),
                        resource_type_rust_type,
                        options_names,
                    );
                    ret.push(param_type);
                }
                if ret.len() == 1 {
                    ret.into_iter().next().unwrap()
                } else {
                    format!("({})", ret.join(", "))
                }
            }
        }
    } else if representations.is_empty() {
        let mut ret = Vec::new();
        for param in &input.params {
            let (param_type, _annotations) = param_rust_type(
                &container,
                param,
                &Config::default(),
                resource_type_rust_type,
                options_names,
            );
            ret.push(param_type);
        }
        if ret.len() == 1 {
            ret.into_iter().next().unwrap()
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

fn format_arg_doc(name: &str, doc: Option<&crate::ast::Doc>, config: &Config) -> Vec<String> {
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

fn apply_map_fn(map_fn: Option<&str>, ret: &str, nillable: bool) -> String {
    if let Some(map_fn) = map_fn {
        if !nillable {
            if map_fn.starts_with('|') {
                format!("({})({})", map_fn, ret)
            } else {
                format!("{}({})", map_fn, ret)
            }
        } else {
            format!("{}.map({})", ret, map_fn)
        }
    } else {
        ret.to_string()
    }
}

fn serialize_representation_def(
    def: &RepresentationDef,
    config: &Config,
    options_names: &HashMap<Options, String>,
) -> Vec<String> {
    let mut lines = vec![];
    fn process_param(
        param: &Param,
        container: &ParamContainer,
        config: &Config,
        cb: impl Fn(&str, &str, &str) -> String,
        options_names: &HashMap<Options, String>,
    ) -> Vec<String> {
        let param_name = escape_rust_reserved(param.name.as_str());

        let (param_type, _annotations) = param_rust_type(
            container,
            param,
            config,
            resource_type_rust_type,
            options_names,
        );
        let param_type = readonly_rust_type(&param_type);
        let mut indent = 4;
        let mut lines = vec![];

        let needs_iter = param_type.starts_with("Vec<") || param_type.starts_with("Option<Vec<");
        let is_optional = param_type.starts_with("Option<");

        if is_optional && param.fixed.is_none() {
            lines.push(format!(
                "{:indent$}if let Some({}) = {} {{\n",
                "",
                param_name,
                param_name,
                indent = indent
            ));
            indent += 4;
        }
        if needs_iter && param.fixed.is_none() {
            lines.push(format!(
                "{:indent$}for {} in {} {{\n",
                "", param_name, param_name
            ));
            indent += 4;
        }

        let value = if let Some(fixed) = param.fixed.as_ref() {
            format!("\"{}\"", fixed)
        } else if param.links.is_empty() {
            format!("&{}.to_string()", param_name)
        } else {
            format!("&{}.url().to_string()", param_name)
        };

        lines.push(format!(
            "{:indent$}{}\n",
            "",
            cb(param_type.as_str(), param.name.as_str(), value.as_str()),
            indent = indent
        ));

        if needs_iter && param.fixed.is_none() {
            indent -= 4;
            lines.push(format!("{:indent$}}}\n", "", indent = indent));
        }

        if is_optional && param.fixed.is_none() {
            indent -= 4;
            lines.push(format!("{:indent$}}}\n", "", indent = indent));
        }

        lines
    }

    let container = ParamContainer::Representation(def);

    match def.media_type.as_ref().map(|s| s.to_string()).as_deref() {
        Some("multipart/form-data") => {
            let mp_mod = if !config.r#async {
                "reqwest::blocking"
            } else {
                "reqwest"
            };
            lines.push(format!(
                "let mut form = {}::multipart::Form::new();\n",
                mp_mod
            ));
            for param in def.params.iter() {
                lines.extend(process_param(
                    param,
                    &container,
                    config,
                    |param_type, name, value| {
                        format!(
                            "form = form.part(\"{}\", {});",
                            name,
                            if let Some(convert_to_multipart) = config
                                .convert_to_multipart
                                .as_ref()
                                .and_then(|x| x(param_type, value))
                            {
                                convert_to_multipart
                            } else {
                                format!(
                                    "{}::multipart::Part::text({})",
                                    mp_mod,
                                    value.strip_prefix('&').unwrap_or(value)
                                )
                            }
                        )
                    },
                    options_names,
                ));
            }
            lines.push("req = req.multipart(form);\n".to_string());
        }
        Some("application/x-www-form-urlencoded") => {
            lines.push(
                "let mut serializer = form_urlencoded::Serializer::new(String::new());\n"
                    .to_string(),
            );
            for param in def.params.iter() {
                lines.extend(process_param(param, &container, config, |r#type, name, value| {
                    if r#type.contains("[") {
                        format!("for value in {} {{ serializer.append_pair(\"{}\", &value.to_string()); }}", value.strip_prefix("&").unwrap().strip_suffix(".to_string()").unwrap(), name)
                    } else {
                        format!("serializer.append_pair(\"{}\", {});", name, value)
                    }
                }, options_names));
            }
            lines.push("req = req.header(reqwest::header::CONTENT_TYPE, \"application/x-www-form-urlencoded\");\n".to_string());
            lines.push("req = req.body(serializer.finish());\n".to_string());
        }
        Some("application/json") => {
            lines.push("let mut o = serde_json::Value::Object::new();".to_string());

            for param in def.params.iter() {
                lines.extend(process_param(
                    param,
                    &container,
                    config,
                    |_type, name, value| format!("o.insert(\"{}\", {});", name, value),
                    options_names,
                ));
            }

            lines.push("req = req.json(&o);\n".to_string());
        }
        o => {
            panic!("unsupported media type {:?}", o);
        }
    }
    lines
}

fn generate_method(
    input: &Method,
    parent_id: &str,
    config: &Config,
    options_names: &HashMap<Options, String>,
) -> Vec<String> {
    let mut lines = generate_method_representation(input, parent_id, config, options_names);

    for response in input.responses.iter() {
        if response.representations.iter().any(|r| {
            r.media_type().as_ref().map(|s| s.to_string()).as_deref() == Some(crate::WADL_MIME_TYPE)
        }) {
            lines.extend(generate_method_wadl(input, parent_id, config))
        }
    }

    lines
}

fn generate_method_wadl(input: &Method, parent_id: &str, config: &Config) -> Vec<String> {
    let mut lines = vec![];

    let name = input.id.as_str();
    let name = name
        .strip_prefix(format!("{}-", parent_id).as_str())
        .unwrap_or(name);
    let name = snake_case_name(name);

    let async_prefix = if config.r#async { "async " } else { "" };

    lines.push(format!("    pub {}fn {}_wadl<'a>(&self, client: &'a dyn {}) -> std::result::Result<wadl::ast::Resource, wadl::Error> {{\n", async_prefix, name, config.client_trait_name()));

    lines.push("        let mut url_ = self.url().clone();\n".to_string());
    for param in input
        .request
        .params
        .iter()
        .filter(|p| p.style == ParamStyle::Query)
    {
        if let Some(fixed) = param.fixed.as_ref() {
            assert!(!param.repeating);
            lines.push(format!(
                "        url_.query_pairs_mut().append_pair(\"{}\", \"{}\");\n",
                param.name, fixed
            ));
        }
    }

    lines.push("\n".to_string());

    let method = input.name.as_str();
    if config.r#async {
        lines.push(format!(
            "        let mut req = client.request(reqwest::Method::{}, url_).await;\n",
            method
        ));
    } else {
        lines.push(format!(
            "        let mut req = client.request(reqwest::Method::{}, url_);\n",
            method
        ));
    }

    lines.push(format!(
        "        req = req.header(reqwest::header::ACCEPT, \"{}\");\n",
        crate::WADL_MIME_TYPE
    ));

    lines.push("\n".to_string());

    if config.r#async {
        lines.push("        let wadl: wadl::ast::Application = req.send().await?.error_for_status()?.text().await?.parse()?;\n".to_string());
    } else {
        lines.push("        let wadl: wadl::ast::Application = req.send()?.error_for_status()?.text()?.parse()?;\n".to_string());
    }
    lines.push(
        "        let resource = wadl.get_resource_by_href(self.url()).unwrap();\n".to_string(),
    );

    lines.push("        Ok(resource.clone())\n".to_string());

    lines.push("    }\n".to_string());

    lines.push("\n".to_string());

    lines
}

fn generate_method_representation(
    input: &Method,
    parent_id: &str,
    config: &Config,
    options_names: &HashMap<Options, String>,
) -> Vec<String> {
    let mut lines = vec![];

    let name = input.id.as_str();
    let name = name
        .strip_prefix(format!("{}-", parent_id).as_str())
        .unwrap_or(name);
    let name = snake_case_name(name);

    let (ret_type, map_fn) = if input.responses.is_empty() {
        ("()".to_string(), None)
    } else {
        assert_eq!(1, input.responses.len(), "expected 1 response for {}", name);
        let mut return_type =
            rust_type_for_response(input, &input.responses[0], input.id.as_str(), options_names);
        let map_fn = if let Some((map_type, map_fn)) = config
            .map_type_for_response
            .as_ref()
            .and_then(|r| r(&name, &return_type, config))
        {
            return_type = map_type;
            Some(map_fn)
        } else {
            None
        };
        (return_type, map_fn)
    };

    let visibility = config
        .method_visibility
        .as_ref()
        .and_then(|x| x(&name, &ret_type))
        .unwrap_or("pub".to_string());

    let mut line = format!(
        "    {}{}fn {}<'a>(&self, client: &'a dyn {}",
        if visibility.is_empty() {
            "".to_string()
        } else {
            format!("{} ", visibility)
        },
        if config.r#async { "async " } else { "" },
        name,
        config.client_trait_name()
    );

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
            Representation::Definition(_) => {}
            Representation::Reference(r) => {
                let id = camel_case_name(r.id().unwrap());
                line.push_str(format!(", representation: &{}", id).as_str());
            }
        }
    }

    let container = ParamContainer::Request(input, &input.request);
    for param in &params {
        if param.fixed.is_some() {
            continue;
        }
        let (param_type, _annotations) = param_rust_type(
            &container,
            param,
            config,
            resource_type_rust_type,
            options_names,
        );
        let param_type = readonly_rust_type(param_type.as_str());
        let param_name = param.name.clone();
        let param_name = escape_rust_reserved(param_name.as_str());

        line.push_str(format!(", {}: {}", param_name, param_type).as_str());

        lines.extend(format_arg_doc(param_name, param.doc.as_ref(), config));
    }
    line.push_str(") -> std::result::Result<");
    line.push_str(ret_type.as_str());

    line.push_str(", wadl::Error> {\n");
    lines.push(line);

    assert!(input
        .request
        .params
        .iter()
        .all(|p| [ParamStyle::Header, ParamStyle::Query].contains(&p.style)));

    lines.push("        let mut url_ = self.url().clone();\n".to_string());
    for param in input
        .request
        .params
        .iter()
        .filter(|p| p.style == ParamStyle::Query)
    {
        if let Some(fixed) = param.fixed.as_ref() {
            assert!(!param.repeating);
            lines.push(format!(
                "        url_.query_pairs_mut().append_pair(\"{}\", \"{}\");\n",
                param.name, fixed
            ));
        } else {
            let param_name = param.name.as_str();
            let param_name = snake_case_name(param_name);
            let param_name = escape_rust_reserved(param_name.as_str());
            let (param_type, _annotations) = param_rust_type(
                &container,
                param,
                config,
                resource_type_rust_type,
                options_names,
            );
            let value = if !param.links.is_empty() {
                format!("&{}.url().to_string()", param_name)
            } else {
                format!("&{}.to_string()", param_name)
            };

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
    if config.r#async {
        lines.push(format!(
            "        let mut req = client.request(reqwest::Method::{}, url_).await;\n",
            method
        ));
    } else {
        lines.push(format!(
            "        let mut req = client.request(reqwest::Method::{}, url_);\n",
            method
        ));
    }

    for representation in &input.request.representations {
        match representation {
            Representation::Definition(ref d) => {
                lines.extend(indent(
                    2,
                    serialize_representation_def(d, config, options_names).into_iter(),
                ));
            }
            Representation::Reference(_r) => {
                // TODO(jelmer): Support non-JSON representations
                lines.push("        req = req.json(&representation);\n".to_string());
            }
        };
    }

    let response_mime_types = input
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

    if !response_mime_types.is_empty() {
        lines.push(format!(
            "        req = req.header(reqwest::header::ACCEPT, \"{}\");\n",
            response_mime_types
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
            let param_name = snake_case_name(param_name);
            let param_name = escape_rust_reserved(param_name.as_str());

            format!("&{}.to_string()", param_name)
        };

        lines.push(format!(
            "        req = req.header(\"{}\", {});\n",
            param.name, value
        ));
    }

    lines.push("\n".to_string());
    if config.r#async {
        lines.push("        let resp = req.send().await?;\n".to_string());
    } else {
        lines.push("        let resp = req.send()?;\n".to_string());
    }

    lines.push("        match resp.status() {\n".to_string());

    let serialize_return_types = |return_types: Vec<(String, bool)>| {
        if return_types.is_empty() {
            "Ok(())".to_string()
        } else if return_types.len() == 1 {
            format!(
                "Ok({})",
                apply_map_fn(map_fn.as_deref(), &return_types[0].0, !return_types[0].1)
            )
        } else {
            let v = format!(
                "({})",
                return_types
                    .iter()
                    .map(|x| x.0.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            format!("Ok({})", apply_map_fn(map_fn.as_deref(), &v, false))
        }
    };

    for response in input.responses.iter() {
        let mut return_types = vec![];

        for param in response.params.iter() {
            match &param.style {
                ParamStyle::Header => {
                    if !param.links.is_empty() {
                        let r = &param.links[0].resource_type.as_ref().unwrap();
                        if !config.nillable(param) {
                            return_types.push((
                                format!(
                                    "{}(resp.headers().get(\"{}\")?.to_str()?.parse().unwrap())",
                                    resource_type_rust_type(r),
                                    param.name
                                ),
                                true,
                            ));
                        } else {
                            return_types.push((format!(
                                "resp.headers().get(\"{}\").map(|x| {}(x.to_str().unwrap().parse().unwrap()))",
                                param.name,
                                resource_type_rust_type(r),
                            ), false));
                        }
                    } else {
                        todo!(
                            "header param type {:?} for {} in {:?}",
                            param.r#type,
                            param.name,
                            input.id
                        );
                    }
                }
                t => todo!("param style {:?}", t),
            }
        }

        // TODO(jelmer): match on media type
        if let Some(status) = response.status {
            lines.push(format!(
                "            s if s.as_u16() == reqwest::StatusCode::{} => {{\n",
                status
            ));
        } else {
            lines.push("            s if s.is_success() => {\n".to_string());
        }

        if !response.representations.is_empty() {
            lines.push("                let content_type: Option<mime::Mime> = resp.headers().get(reqwest::header::CONTENT_TYPE).map(|x| x.to_str().unwrap()).map(|x| x.parse().unwrap());\n".to_string());
            lines.push(
                "                match content_type.as_ref().map(|x| x.essence_str()) {\n"
                    .to_string(),
            );
            for representation in response.representations.iter() {
                let media_type = representation
                    .media_type()
                    .unwrap_or(&mime::APPLICATION_JSON);
                lines.push(format!(
                    "                    Some(\"{}\") => {{\n",
                    media_type
                ));
                let t = match representation {
                    Representation::Definition(_) => None,
                    Representation::Reference(r) => {
                        let rt = representation_rust_type(r);

                        Some((
                            format!(
                                "resp.json::<{}>(){}?",
                                rt,
                                if config.r#async { ".await" } else { "" }
                            ),
                            true,
                        ))
                    }
                };
                if let Some(t) = t {
                    let mut return_types = return_types.clone();
                    return_types.insert(0, t);
                    lines.push(format!(
                        "                             {}\n",
                        serialize_return_types(return_types)
                    ));
                } else {
                    lines.push("                        unimplemented!();\n".to_string());
                }
                lines.push("                        }\n".to_string());
            }
            lines.push(
                "                    _ => { Err(wadl::Error::UnhandledContentType(content_type)) }\n"
                    .to_string(),
            );
            lines.push("                }\n".to_string());
        } else {
            lines.push(format!(
                "                {}\n",
                serialize_return_types(return_types)
            ));
        }

        lines.push("            }\n".to_string());
    }
    if input.responses.is_empty() {
        lines.push("            s if s.is_success() => Ok(()),\n".to_string());
    }
    lines.push("            s => Err(wadl::Error::UnhandledStatus(s))\n".to_string());
    lines.push("        }\n".to_string());
    lines.push("    }\n".to_string());
    lines.push("\n".to_string());

    if let Some(extend_method) = config.extend_method.as_ref() {
        lines.extend(extend_method(parent_id, &name, &ret_type, config));
    }

    lines
}

fn generate_resource_type(
    input: &ResourceType,
    config: &Config,
    options_names: &HashMap<Options, String>,
) -> Vec<String> {
    let mut lines = vec![];

    for doc in &input.docs {
        lines.extend(generate_doc(doc, 0, config));
    }

    let name = input.id.as_str();
    let name = camel_case_name(name);

    let visibility = config
        .resource_type_visibility
        .as_ref()
        .and_then(|x| x(name.as_str()))
        .unwrap_or("pub".to_string());

    lines.push(format!(
        "{}struct {} (reqwest::Url);\n",
        if visibility.is_empty() {
            "".to_string()
        } else {
            format!("{} ", visibility)
        },
        name
    ));

    lines.push("\n".to_string());

    lines.push(format!("impl {} {{\n", name));

    for method in &input.methods {
        lines.extend(generate_method(
            method,
            input.id.as_str(),
            config,
            options_names,
        ));
    }

    lines.push("}\n".to_string());
    lines.push("\n".to_string());
    lines.push(format!("impl wadl::Resource for {} {{\n", name));
    lines.push("    fn url(&self) -> &reqwest::Url {\n".to_string());
    lines.push("        &self.0\n".to_string());
    lines.push("    }\n".to_string());
    lines.push("}\n".to_string());
    lines.push("\n".to_string());
    lines
}

#[derive(Default)]
#[allow(clippy::type_complexity)]
/// Configuration for code generation
pub struct Config {
    /// Whether to generate async code
    pub r#async: bool,

    /// Based on the listed type and name of a parameter, determine the rust type
    pub override_type_name:
        Option<Box<dyn Fn(&ParamContainer, &str, &str, &Config) -> Option<String>>>,

    /// Support renaming param accessor functions
    pub param_accessor_rename: Option<Box<dyn Fn(&str, &str) -> Option<String>>>,

    /// Whether to strip code examples from the docstrings
    ///
    /// This is useful if the code examples are not valid rust code.
    pub strip_code_examples: bool,

    /// Generate custom trait implementations for representations
    pub generate_representation_traits: Option<
        Box<dyn Fn(&RepresentationDef, &str, &RepresentationDef, &Config) -> Option<Vec<String>>>,
    >,

    /// Return the visibility of a representation
    pub representation_visibility: Option<Box<dyn Fn(&str) -> Option<String>>>,

    /// Return the visibility of a representation accessor
    pub accessor_visibility: Option<Box<dyn Fn(&str, &str) -> Option<String>>>,

    /// Return the visibility of a resource type
    pub resource_type_visibility: Option<Box<dyn Fn(&str) -> Option<String>>>,

    /// Map a method response type to a different type and a function to map the response
    pub map_type_for_response: Option<Box<dyn Fn(&str, &str, &Config) -> Option<(String, String)>>>,

    /// Map an accessor function name to a different type
    pub map_type_for_accessor: Option<Box<dyn Fn(&str) -> Option<(String, String)>>>,

    /// Extend the generated accessor
    pub extend_accessor: Option<Box<dyn Fn(&Param, &'_ str, &'_ str, &Config) -> Vec<String>>>,

    /// Extend the generated method
    pub extend_method: Option<Box<dyn Fn(&str, &str, &str, &Config) -> Vec<String>>>,

    /// Retrieve visibility for a method
    pub method_visibility: Option<Box<dyn Fn(&str, &str) -> Option<String>>>,

    /// Return whether a param is deprecated
    pub deprecated_param: Option<Box<dyn Fn(&Param) -> bool>>,

    /// Return the name for an enum representation a set of options
    ///
    /// The callback can be used to determine if the name is already taken.
    pub options_enum_name: Option<Box<dyn Fn(&Param, Box<dyn Fn(&str) -> bool>) -> String>>,

    /// Reformat a docstring; should already be in markdown
    pub reformat_docstring: Option<Box<dyn Fn(&str) -> String>>,

    /// Convert a string to a multipart Part, given a type name and value
    pub convert_to_multipart: Option<Box<dyn Fn(&str, &str) -> Option<String>>>,

    /// Check whether a parameter can be nil
    pub nillable_param: Option<Box<dyn Fn(&Param) -> bool>>,
}

impl Config {
    /// Return identifier of the wadl client
    pub fn client_trait_name(&self) -> &'static str {
        if self.r#async {
            "wadl::r#async::Client"
        } else {
            "wadl::blocking::Client"
        }
    }

    /// Check whether the parameter is can be nil
    pub fn nillable(&self, param: &Param) -> bool {
        if let Some(nillable_param) = self.nillable_param.as_ref() {
            nillable_param(param)
        } else {
            !param.required
        }
    }
}

fn enum_rust_value(option: &str) -> String {
    let name = camel_case_name(option.replace(' ', "-").as_str());

    // Now, strip all characters not allowed in rust identifiers
    let name = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    // If the identifier starts with a digit, prefix it with '_' to make it a valid identifier
    if name.chars().next().unwrap().is_numeric() {
        format!("_{}", name)
    } else {
        name
    }
}

fn generate_options(name: &str, options: &crate::ast::Options) -> Vec<String> {
    let mut lines = vec![];

    lines.push("#[derive(Debug, Clone, Copy, PartialEq, Eq, std::hash::Hash, serde::Serialize, serde::Deserialize)]\n".to_string());
    lines.push(format!("pub enum {} {{\n", name));

    let mut option_map = HashMap::new();

    for option in options.keys() {
        let rust_name = enum_rust_value(option);
        lines.push(format!("    #[serde(rename = \"{}\")]\n", option));
        lines.push(format!("    {},\n", rust_name));
        option_map.insert(option, rust_name);
    }
    lines.push("}\n".to_string());
    lines.push("\n".to_string());

    lines.push(format!("impl std::fmt::Display for {} {{\n", name));
    lines.push(
        "    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n".to_string(),
    );
    lines.push("        match self {\n".to_string());
    for (option, rust_name) in option_map {
        lines.push(format!(
            "            {}::{} => write!(f, \"{}\"),\n",
            name, rust_name, option
        ));
    }
    lines.push("        }\n".to_string());
    lines.push("    }\n".to_string());
    lines.push("}\n".to_string());
    lines
}

fn options_rust_enum_name(param: &Param, options: &HashMap<Options, String>) -> String {
    let mut name = camel_case_name(param.name.as_str());
    while options.values().any(|v| v == &name) {
        name = format!("{}_", name);
    }
    name
}

/// Generate code from a WADL application definition.
///
/// This function generates Rust code from a WADL application definition.
/// The generated code includes Rust types for the representations and
/// resource types defined in the WADL application, as well as methods
/// for interacting with the resources.
///
/// # Arguments
/// * `app` - The WADL application definition.
/// * `config` - Configuration for the code generation.
pub fn generate(app: &Application, config: &Config) -> String {
    let mut lines = vec![];

    let mut options = HashMap::new();

    for param in app.iter_all_params() {
        if let Some(os) = &param.options {
            if options.contains_key(os) {
                continue;
            }
            let name = if let Some(enum_name_fn) = config.options_enum_name.as_ref() {
                let cb_options = options.clone();
                let name = enum_name_fn(
                    param,
                    Box::new(move |name: &str| -> bool { cb_options.values().any(|v| v == name) }),
                );
                let taken = options
                    .iter()
                    .filter_map(|(k, v)| if v == &name { Some(k) } else { None })
                    .collect::<Vec<_>>();
                if !taken.is_empty() {
                    panic!(
                        "Enum name {} is already taken by {:?} ({:?})",
                        name, taken, options
                    );
                }
                name
            } else {
                options_rust_enum_name(param, &options)
            };
            let enum_lines = generate_options(name.as_str(), os);
            options.insert(os.clone(), name);
            lines.extend(enum_lines);
        }
    }

    for doc in &app.docs {
        lines.extend(generate_doc(doc, 0, config));
    }

    for representation in &app.representations {
        lines.extend(generate_representation(representation, config, &options));
    }

    for resource_type in &app.resource_types {
        lines.extend(generate_resource_type(resource_type, config, &options));
    }

    lines.concat()
}

fn indent(indent: usize, lines: impl Iterator<Item = String>) -> impl Iterator<Item = String> {
    lines.map(move |line| format!("{}{}", " ".repeat(indent * 4), line))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_camel_case_name() {
        assert_eq!(camel_case_name("foo-bar"), "FooBar");
        assert_eq!(camel_case_name("foo-bar-baz"), "FooBarBaz");
        assert_eq!(camel_case_name("foo-bar-baz-quux"), "FooBarBazQuux");
        assert_eq!(camel_case_name("_foo-bar"), "_fooBar");
        assert_eq!(camel_case_name("service-root-json"), "ServiceRootJson");
        assert_eq!(camel_case_name("get-some-URL"), "GetSomeURL");
    }

    #[test]
    fn test_generate_empty() {
        let input = crate::ast::Application {
            docs: vec![],
            representations: vec![],
            resource_types: vec![],
            resources: vec![],
            grammars: vec![],
        };
        let config = Config::default();
        let lines = generate(&input, &config);
        assert_eq!(lines, "".to_string());
    }

    #[test]
    fn test_enum_rust_value() {
        assert_eq!(enum_rust_value("foo"), "Foo");
        assert_eq!(enum_rust_value("foo bar"), "FooBar");
        assert_eq!(enum_rust_value("foo bar blah"), "FooBarBlah");
        assert_eq!(enum_rust_value("foo-bar"), "FooBar");
    }

    #[test]
    fn test_snake_case_name() {
        assert_eq!(snake_case_name("F"), "f");
        assert_eq!(snake_case_name("FooBar"), "foo_bar");
        assert_eq!(snake_case_name("FooBarBaz"), "foo_bar_baz");
        assert_eq!(snake_case_name("FooBarBazQuux"), "foo_bar_baz_quux");
        assert_eq!(snake_case_name("_FooBar"), "_foo_bar");
        assert_eq!(snake_case_name("ServiceRootJson"), "service_root_json");
        assert_eq!(snake_case_name("GetSomeURL"), "get_some_url");
    }

    #[test]
    fn test_strip_code_examples() {
        let input = r#"This is a test
```python
def foo():
    pass
```

This is another test
```python
def bar():
    pass
```
"#;
        let expected = r#"This is a test

This is another test"#;
        assert_eq!(strip_code_examples(input.to_string()), expected);
    }

    #[test]
    fn test_format_doc_plain() {
        let doc = Doc {
            title: None,
            lang: None,
            content: "This is a test".to_string(),
            xmlns: None,
        };

        assert_eq!(
            format_doc(&doc, &Config::default()),
            "This is a test".to_string()
        );
    }

    #[test]
    fn test_format_doc_html() {
        let doc = Doc {
            title: None,
            lang: None,
            content: "<p>This is a test</p>".to_string(),
            xmlns: Some("http://www.w3.org/1999/xhtml".parse().unwrap()),
        };

        assert_eq!(
            format_doc(&doc, &Config::default()),
            "This is a test".to_string()
        );
    }

    #[test]
    fn test_format_doc_html_link() {
        let doc = Doc {
            title: None,
            lang: None,
            content: "<p>This is a <a href=\"https://example.com\">test</a></p>".to_string(),
            xmlns: Some("http://www.w3.org/1999/xhtml".parse().unwrap()),
        };

        assert_eq!(
            format_doc(&doc, &Config::default()),
            "This is a [test](https://example.com)".to_string()
        );
    }

    #[test]
    fn test_generate_doc_plain() {
        let doc = Doc {
            title: Some("Foo".to_string()),
            lang: None,
            content: "This is a test".to_string(),
            xmlns: None,
        };

        assert_eq!(
            generate_doc(&doc, 0, &Config::default()),
            vec![
                "/// # Foo\n".to_string(),
                "///\n".to_string(),
                "/// This is a test\n".to_string(),
            ]
        );
    }

    #[test]
    fn test_generate_doc_html() {
        let doc = Doc {
            title: Some("Foo".to_string()),
            lang: None,
            content: "<p>This is a test</p>".to_string(),
            xmlns: Some("http://www.w3.org/1999/xhtml".parse().unwrap()),
        };

        assert_eq!(
            generate_doc(&doc, 0, &Config::default()),
            vec![
                "/// # Foo\n".to_string(),
                "///\n".to_string(),
                "/// This is a test\n".to_string(),
            ]
        );
    }

    #[test]
    fn test_generate_doc_multiple_lines() {
        let doc = Doc {
            title: Some("Foo".to_string()),
            lang: None,
            content: "This is a test\n\nThis is another test".to_string(),
            xmlns: None,
        };

        assert_eq!(
            generate_doc(&doc, 0, &Config::default()),
            vec![
                "/// # Foo\n".to_string(),
                "///\n".to_string(),
                "/// This is a test\n".to_string(),
                "///\n".to_string(),
                "/// This is another test\n".to_string(),
            ]
        );
    }

    #[test]
    fn test_resource_type_rust_type() {
        use std::str::FromStr;
        let rt = ResourceTypeRef::from_str("https://api.launchpad.net/1.0/#person").unwrap();
        assert_eq!(resource_type_rust_type(&rt), "Person");
    }

    #[test]
    fn test_param_rust_type() {
        use std::str::FromStr;
        let rt = ResourceTypeRef::from_str("https://api.launchpad.net/1.0/#person").unwrap();
        let mut param = Param {
            name: "person".to_string(),
            r#type: "string".to_string(),
            required: true,
            repeating: false,
            fixed: None,
            doc: None,
            options: None,
            id: None,
            style: ParamStyle::Plain,
            path: None,
            links: vec![crate::ast::Link {
                resource_type: Some(rt),
                relation: None,
                reverse_relation: None,
                doc: None,
            }],
        };

        let method = Method {
            docs: vec![],
            id: "getPerson".to_string(),
            name: "getPerson".to_string(),
            request: Request {
                docs: vec![],
                params: vec![param.clone()],
                representations: vec![],
            },
            responses: vec![Response {
                status: None,
                docs: vec![],
                params: vec![param.clone()],
                representations: vec![],
            }],
        };

        let container = ParamContainer::Request(&method, &method.request);

        let (param_type, _) = param_rust_type(
            &container,
            &param,
            &Config::default(),
            resource_type_rust_type,
            &HashMap::new(),
        );
        assert_eq!(param_type, "Person");

        param.required = false;
        let (param_type, _) = param_rust_type(
            &container,
            &param,
            &Config::default(),
            resource_type_rust_type,
            &HashMap::new(),
        );
        assert_eq!(param_type, "Option<Person>");

        param.repeating = true;
        param.required = true;
        let (param_type, _) = param_rust_type(
            &container,
            &param,
            &Config::default(),
            resource_type_rust_type,
            &HashMap::new(),
        );
        assert_eq!(param_type, "Vec<Person>");

        param.repeating = false;
        param.r#type = "string".to_string();
        param.links = vec![];
        let (param_type, _) = param_rust_type(
            &container,
            &param,
            &Config::default(),
            resource_type_rust_type,
            &HashMap::new(),
        );
        assert_eq!(param_type, "String");

        param.r#type = "binary".to_string();
        let (param_type, _) = param_rust_type(
            &container,
            &param,
            &Config::default(),
            resource_type_rust_type,
            &HashMap::new(),
        );
        assert_eq!(param_type, "Vec<u8>");

        param.r#type = "xsd:date".to_string();
        let (param_type, _) = param_rust_type(
            &container,
            &param,
            &Config::default(),
            resource_type_rust_type,
            &HashMap::new(),
        );
        assert_eq!(param_type, "chrono::NaiveDate");

        param.r#type = "string".to_string();
        param.options = Some(Options::from(vec!["one".to_string(), "two".to_string()]));
        let (param_type, _) = param_rust_type(
            &container,
            &param,
            &Config::default(),
            resource_type_rust_type,
            &maplit::hashmap! {
                Options::from(vec!["one".to_string(), "two".to_string()]) => "MyOptions".to_string(),
            },
        );
        assert_eq!(param_type, "MyOptions");
    }

    #[test]
    fn test_readonly_rust_type() {
        assert_eq!(readonly_rust_type("String"), "&str");
        assert_eq!(readonly_rust_type("Vec<String>"), "&[String]");
        assert_eq!(
            readonly_rust_type("Option<Vec<String>>"),
            "Option<&[String]>"
        );
        assert_eq!(readonly_rust_type("Option<String>"), "Option<&str>");
        assert_eq!(readonly_rust_type("usize"), "&usize");
    }

    #[test]
    fn test_escape_rust_reserved() {
        assert_eq!(escape_rust_reserved("type"), "r#type");
        assert_eq!(escape_rust_reserved("match"), "r#match");
        assert_eq!(escape_rust_reserved("move"), "r#move");
        assert_eq!(escape_rust_reserved("use"), "r#use");
        assert_eq!(escape_rust_reserved("loop"), "r#loop");
        assert_eq!(escape_rust_reserved("continue"), "r#continue");
        assert_eq!(escape_rust_reserved("break"), "r#break");
        assert_eq!(escape_rust_reserved("fn"), "r#fn");
        assert_eq!(escape_rust_reserved("struct"), "r#struct");
        assert_eq!(escape_rust_reserved("enum"), "r#enum");
        assert_eq!(escape_rust_reserved("trait"), "r#trait");
        assert_eq!(escape_rust_reserved("impl"), "r#impl");
        assert_eq!(escape_rust_reserved("pub"), "r#pub");
        assert_eq!(escape_rust_reserved("as"), "r#as");
        assert_eq!(escape_rust_reserved("const"), "r#const");
        assert_eq!(escape_rust_reserved("let"), "r#let");
        assert_eq!(escape_rust_reserved("foo"), "foo");
    }

    #[test]
    fn test_representation_rust_type() {
        let rt = RepresentationRef::Id("person".to_string());
        assert_eq!(representation_rust_type(&rt), "Person");
    }

    #[test]
    fn test_generate_representation() {
        let input = RepresentationDef {
            media_type: Some("application/json".parse().unwrap()),
            element: None,
            profile: None,
            docs: vec![],
            id: Some("person".to_string()),
            params: vec![
                Param {
                    name: "name".to_string(),
                    r#type: "string".to_string(),
                    style: ParamStyle::Plain,
                    required: true,
                    doc: Some(Doc::new("The name of the person".to_string())),
                    path: None,
                    id: None,
                    repeating: false,
                    fixed: None,
                    links: vec![],
                    options: None,
                },
                Param {
                    name: "age".to_string(),
                    r#type: "xs:int".to_string(),
                    required: true,
                    doc: Some(Doc::new("The age of the person".to_string())),
                    style: ParamStyle::Query,
                    path: None,
                    id: None,
                    repeating: false,
                    fixed: None,
                    links: vec![],
                    options: None,
                },
            ],
        };

        let config = Config::default();

        let lines = generate_representation_struct_json(&input, &config, &HashMap::new());

        assert_eq!(
            lines,
            vec![
                "/// Representation of the `person` resource\n".to_string(),
                "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n"
                    .to_string(),
                "pub struct Person {\n".to_string(),
                "    // was: string\n".to_string(),
                "    /// The name of the person\n".to_string(),
                "    pub name: String,\n".to_string(),
                "\n".to_string(),
                "    // was: xs:int\n".to_string(),
                "    /// The age of the person\n".to_string(),
                "    pub age: i32,\n".to_string(),
                "\n".to_string(),
                "}\n".to_string(),
                "\n".to_string(),
            ]
        );
    }

    #[test]
    fn test_supported_representation_def() {
        let mut d = RepresentationDef {
            media_type: Some(crate::WADL_MIME_TYPE.parse().unwrap()),
            ..Default::default()
        };
        assert!(!supported_representation_def(&d));

        d.media_type = Some(XHTML_MIME_TYPE.parse().unwrap());
        assert!(!supported_representation_def(&d));

        d.media_type = Some("application/json".parse().unwrap());
        assert!(!supported_representation_def(&d));
    }

    #[test]
    fn test_rust_type_for_response() {
        let mut input = Response {
            params: vec![Param {
                id: Some("foo".to_string()),
                name: "foo".to_string(),
                r#type: "string".to_string(),
                style: ParamStyle::Header,
                doc: None,
                required: true,
                repeating: false,
                fixed: None,
                path: None,
                links: Vec::new(),
                options: None,
            }],
            ..Default::default()
        };

        let method = Method {
            name: "GET".to_string(),
            id: "get".to_string(),
            docs: Vec::new(),
            request: Request::default(),
            responses: vec![input.clone()],
        };

        assert_eq!(
            rust_type_for_response(&method, &input, "foo", &HashMap::new()),
            "String".to_string()
        );

        input.params = vec![
            Param {
                id: Some("foo".to_string()),
                name: "foo".to_string(),
                r#type: "string".to_string(),
                style: ParamStyle::Header,
                doc: None,
                required: true,
                repeating: false,
                fixed: None,
                path: None,
                links: Vec::new(),
                options: None,
            },
            Param {
                id: Some("bar".to_string()),
                name: "bar".to_string(),
                r#type: "string".to_string(),
                style: ParamStyle::Header,
                doc: None,
                required: true,
                repeating: false,
                fixed: None,
                path: None,
                links: Vec::new(),
                options: None,
            },
        ];
        assert_eq!(
            rust_type_for_response(&method, &input, "foo", &HashMap::new()),
            "(String, String)".to_string()
        );

        input.params = vec![Param {
            id: Some("foo".to_string()),
            name: "foo".to_string(),
            r#type: "string".to_string(),
            style: ParamStyle::Header,
            doc: None,
            required: true,
            repeating: false,
            fixed: None,
            path: None,
            links: vec![Link {
                relation: None,
                reverse_relation: None,
                resource_type: Some("http://example.com/#foo".parse().unwrap()),
                doc: None,
            }],
            options: None,
        }];
        assert_eq!(
            rust_type_for_response(&method, &input, "foo", &HashMap::new()),
            "Foo".to_string()
        );

        input.params = vec![Param {
            id: Some("foo".to_string()),
            name: "foo".to_string(),
            r#type: "string".to_string(),
            style: ParamStyle::Header,
            doc: None,
            required: true,
            repeating: false,
            fixed: None,
            path: None,
            links: vec![Link {
                relation: None,
                reverse_relation: None,
                resource_type: Some("http://example.com/#foo".parse().unwrap()),
                doc: None,
            }],
            options: None,
        }];
        assert_eq!(
            rust_type_for_response(&method, &input, "foo", &HashMap::new()),
            "Foo".to_string()
        );

        input.params = vec![Param {
            id: None,
            name: "foo".to_string(),
            r#type: "string".to_string(),
            style: ParamStyle::Header,
            doc: None,
            required: true,
            repeating: false,
            fixed: None,
            options: None,
            path: None,
            links: vec![Link {
                relation: None,
                reverse_relation: None,
                resource_type: None,
                doc: None,
            }],
        }];
        assert_eq!(
            rust_type_for_response(&method, &input, "foo", &HashMap::new()),
            "url::Url".to_string()
        );
    }

    #[test]
    fn test_format_arg_doc() {
        let config = Config::default();
        assert_eq!(
            format_arg_doc("foo", None, &config),
            vec!["    /// * `foo`\n".to_string()]
        );
        assert_eq!(
            format_arg_doc("foo", Some(&Doc::new("bar".to_string())), &config),
            vec!["    /// * `foo`: bar\n".to_string()]
        );
        assert_eq!(
            format_arg_doc("foo", Some(&Doc::new("bar\nbaz".to_string())), &config),
            vec![
                "    /// * `foo`: bar\n".to_string(),
                "    ///     baz\n".to_string()
            ]
        );
        assert_eq!(
            format_arg_doc("foo", Some(&Doc::new("bar\n\nbaz".to_string())), &config),
            vec![
                "    /// * `foo`: bar\n".to_string(),
                "    ///\n".to_string(),
                "    ///     baz\n".to_string()
            ]
        );
    }

    #[test]
    fn test_apply_map_fn() {
        assert_eq!(apply_map_fn(None, "x", false), "x".to_string());
        assert_eq!(
            apply_map_fn(Some("Some"), "x", false),
            "Some(x)".to_string()
        );
        assert_eq!(
            apply_map_fn(Some("Some"), "x", true),
            "x.map(Some)".to_string()
        );
        assert_eq!(
            apply_map_fn(Some("|y|y+1"), "x", false),
            "(|y|y+1)(x)".to_string()
        );
        assert_eq!(
            apply_map_fn(Some("|y|y+1"), "x", true),
            "x.map(|y|y+1)".to_string()
        );
    }

    #[test]
    fn test_generate_method() {
        let input = Method {
            id: "foo".to_string(),
            name: "GET".to_string(),
            docs: vec![],
            request: Request {
                docs: vec![],
                params: vec![],
                representations: vec![],
            },
            responses: vec![],
        };
        let config = Config::default();
        let lines = generate_method(&input, "bar", &config, &HashMap::new());
        assert_eq!(lines, vec![
        "    pub fn foo<'a>(&self, client: &'a dyn wadl::blocking::Client) -> std::result::Result<(), wadl::Error> {\n".to_string(),
        "        let mut url_ = self.url().clone();\n".to_string(),
        "\n".to_string(),
        "        let mut req = client.request(reqwest::Method::GET, url_);\n".to_string(),
        "\n".to_string(),
        "        let resp = req.send()?;\n".to_string(),
        "        match resp.status() {\n".to_string(),
        "            s if s.is_success() => Ok(()),\n".to_string(),
        "            s => Err(wadl::Error::UnhandledStatus(s))\n".to_string(),
        "        }\n".to_string(),
        "    }\n".to_string(),
        "\n".to_string(),
    ]);
    }

    #[test]
    fn test_generate_resource_type() {
        let input = ResourceType {
            id: "foo".to_string(),
            docs: vec![],
            methods: vec![],
            query_type: mime::APPLICATION_JSON,
            params: vec![],
            subresources: vec![],
        };
        let config = Config::default();
        let lines = generate_resource_type(&input, &config, &HashMap::new());
        assert_eq!(
            lines,
            vec![
                "pub struct Foo (reqwest::Url);\n".to_string(),
                "\n".to_string(),
                "impl Foo {\n".to_string(),
                "}\n".to_string(),
                "\n".to_string(),
                "impl wadl::Resource for Foo {\n".to_string(),
                "    fn url(&self) -> &reqwest::Url {\n".to_string(),
                "        &self.0\n".to_string(),
                "    }\n".to_string(),
                "}\n".to_string(),
                "\n".to_string(),
            ]
        );
    }
}
