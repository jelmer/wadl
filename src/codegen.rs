use crate::ast::*;

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

fn generate_doc(input: &Doc) -> Vec<String> {
    let mut lines: Vec<String> = vec![];

    if let Some(title) = input.title.as_ref() {
        lines.extend(vec![format!("/// #{}\n", title), "///\n".to_string()]);
    }

    lines.push(if let Some(xmlns) = &input.xmlns {
        let lang = match xmlns.as_str() {
            "http://www.w3.org/2001/XMLSchema" => "xml",
            "http://www.w3.org/1999/xhtml" => "html",
            _ => {
                log::warn!("Unknown xmlns: {}", xmlns);
                ""
            }
        };
        format!("/// ```{}\n", lang)
    } else {
        "/// ```\n".to_string()
    });

    lines.extend(input.content.lines().map(|line| format!("/// {}\n", line)));
    lines.push("/// ```\n".to_string());
    lines
}

fn generate_representation(input: &Representation, config: &Config) -> Vec<String> {
    let mut lines = vec![];
    for doc in &input.docs {
        lines.extend(generate_doc(doc));
    }

    if input.media_type == Some(mime::APPLICATION_JSON) {
        lines.extend(generate_representation_struct_json(input, config));
    } else {
        panic!("Unknown media type: {:?}", input.media_type);
    }

    lines
}

fn generate_representation_struct_json(input: &Representation, config: &Config) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let name = input.id.as_ref().unwrap().as_str();
    let name = camel_case_name(name);

    lines.push(
        "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n".to_string(),
    );
    lines.push(format!("pub struct {} {{\n", name));

    for param in &input.params {
        let mut param_name = snake_case_name(param.name.as_str());

        assert!(param.id.is_none());
        assert!(param.fixed.is_none());

        let (mut param_type, comment) = match &param.r#type {
            TypeRef::Simple(name) => (
                match name.as_str() {
                    "xsd:date" => "chrono::NaiveDate".to_string(),
                    "xsd:dateTime" => "chrono::NaiveDateTime".to_string(),
                    "xsd:duration" => "chrono::Duration".to_string(),
                    "xsd:time" => "chrono::NaiveTime".to_string(),
                    "string" => "String".to_string(),
                    "binary" => "Vec<u8>".to_string(),
                    u => panic!("Unknown type: {}", u),
                },
                format!("was: {}", name),
            ),
            TypeRef::EmptyLink => {
                // This would be a reference to the representation itself
                ("url::Url".to_string(), "was: empty link".to_string())
            }
            TypeRef::ResourceTypeId(id) => {
                ("url::Url".to_string(), format!("resource type id: {}", id))
            }
            TypeRef::ResourceTypeLink(href) => (
                "url::Url".to_string(),
                format!("resource type link: {}", href),
            ),
            TypeRef::Options(options) => {
                // TODO: define an enum for this
                ("String".to_string(), format!("options: {:?}", options))
            }
            TypeRef::NoType => {
                let tn = if let Some(guess_name) = config.guess_type_name.as_ref() {
                    guess_name(param.name.as_str())
                } else {
                    None
                };

                (
                    if let Some(tn) = tn {
                        tn
                    } else {
                        log::warn!("No type for parameter: {}", param.name);
                        "serde_json::Value".to_string()
                    },
                    "no type for parameter in WADL".to_string(),
                )
            }
        };

        if param.repeating {
            param_type = format!("Vec<{}>", param_type);
        }

        if !param.required {
            param_type = format!("Option<{}>", param_type);
        }

        if ["type"].contains(&param_name.as_str()) {
            param_name = format!("r#{}", param_name);
        }

        lines.push(format!("    // {}\n", comment));
        lines.push(format!("    pub {}: {},\n", param_name, param_type));
    }

    lines.push("}\n".to_string());
    lines.push("\n".to_string());

    lines
}

#[derive(Default)]
pub struct Config {
    pub guess_type_name: Option<Box<dyn Fn(&str) -> Option<String>>>,
}

pub fn generate(app: &Application, config: &Config) -> String {
    let mut lines = vec![];

    for doc in &app.docs {
        lines.extend(generate_doc(doc));
    }

    for representation in &app.representations {
        lines.extend(generate_representation(representation, config));
    }

    lines.concat()
}
