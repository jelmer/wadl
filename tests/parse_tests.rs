use wadl::{parse_string, ParseError};

#[test]
fn test_parse_empty_xml() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <application xmlns="http://wadl.dev.java.net/2009/02">
    </application>"#;

    let result = parse_string(xml);
    assert!(result.is_ok());
}

#[test]
fn test_parse_invalid_xml() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <invalid-root>
    </invalid-root>"#;

    let result = parse_string(xml);
    // The parser might successfully parse XML that isn't a valid WADL
    // It will parse but return an application with empty resources
    assert!(result.is_ok());
    let app = result.unwrap();
    assert!(app.resources.is_empty());
}

#[test]
fn test_error_display() {
    let io_error = std::io::Error::new(std::io::ErrorKind::InvalidData, "test error");
    let error = ParseError::Io(io_error);
    let display_string = format!("{}", error);
    assert!(display_string.contains("test error"));
}

#[test]
fn test_minimal_wadl() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <application xmlns="http://wadl.dev.java.net/2009/02">
        <resources base="http://example.com/api/">
            <resource path="users">
                <method name="GET">
                    <response status="200"/>
                </method>
            </resource>
        </resources>
    </application>"#;

    let result = parse_string(xml);
    assert!(result.is_ok());

    let app = result.unwrap();
    assert_eq!(app.resources.len(), 1);
    assert_eq!(app.resources[0].resources.len(), 1);
    assert_eq!(app.resources[0].resources[0].methods.len(), 1);
}

#[test]
fn test_wadl_with_params() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <application xmlns="http://wadl.dev.java.net/2009/02" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
        <resources base="http://example.com/api/">
            <resource path="users/{id}">
                <param name="id" style="template" type="xsd:string" required="true"/>
                <method name="GET">
                    <request>
                        <param name="format" style="query" type="xsd:string" default="json"/>
                    </request>
                    <response status="200"/>
                </method>
            </resource>
        </resources>
    </application>"#;

    let result = parse_string(xml);
    assert!(result.is_ok());

    let app = result.unwrap();
    let resource = &app.resources[0].resources[0];
    assert_eq!(resource.params.len(), 1);
    assert_eq!(resource.params[0].name, "id");

    let method = &resource.methods[0];
    assert_eq!(method.request.params.len(), 1);
    assert_eq!(method.request.params[0].name, "format");
}

#[test]
fn test_param_type_namespaces() {
    use wadl::ast::TypeName;
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <application xmlns="http://wadl.dev.java.net/2009/02" xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <representation id="thing" mediaType="application/json">
            <param name="a" style="plain" type="xs:int"/>
            <param name="b" style="plain" type="binary"/>
            <param name="c" style="plain" xmlns:t="urn:example:types" type="t:Wrapper&lt;i32&gt;"/>
            <param name="d" style="plain"/>
        </representation>
    </application>"#;

    let app = parse_string(xml).unwrap();
    let types = app.representations[0]
        .params
        .iter()
        .map(|p| p.r#type.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        types,
        vec![
            Some(TypeName::xsd("int")),
            Some(TypeName {
                namespace: None,
                name: "binary".to_string()
            }),
            Some(TypeName {
                namespace: Some("urn:example:types".to_string()),
                name: "Wrapper<i32>".to_string()
            }),
            None,
        ]
    );
}

#[test]
fn test_param_type_undeclared_prefix() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <application xmlns="http://wadl.dev.java.net/2009/02">
        <representation id="thing" mediaType="application/json">
            <param name="a" style="plain" type="xs:int"/>
        </representation>
    </application>"#;

    match parse_string(xml) {
        Err(ParseError::UndeclaredPrefix(name)) => assert_eq!(name, "xs:int"),
        other => panic!("unexpected result: {:?}", other),
    }
}
