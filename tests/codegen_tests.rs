#[cfg(feature = "codegen")]
use wadl::ast::{Application, Doc};
#[cfg(feature = "codegen")]
use wadl::codegen::*;

#[cfg(feature = "codegen")]
#[test]
fn test_camel_case_name() {
    assert_eq!(camel_case_name("test"), "Test");
    assert_eq!(camel_case_name("test-name"), "TestName");
    assert_eq!(camel_case_name("test_name"), "TestName");
    assert_eq!(camel_case_name("xml"), "Xml");
}

#[cfg(feature = "codegen")]
#[test]
fn test_snake_case_name() {
    assert_eq!(snake_case_name("Test"), "test");
    assert_eq!(snake_case_name("TestName"), "test_name");
    assert_eq!(snake_case_name("test-name"), "test_name");
    assert_eq!(snake_case_name("XMLParser"), "xmlparser");
}

#[cfg(feature = "codegen")]
#[test]
fn test_generate_doc() {
    let doc = Doc::new("Test documentation".to_string());
    let config = Config::default();
    let result = generate_doc(&doc, 0, &config);
    assert!(!result.is_empty());
}

#[cfg(feature = "codegen")]
#[test]
fn test_generate_empty_application() {
    let app = Application {
        resources: vec![],
        resource_types: vec![],
        docs: vec![],
        grammars: vec![],
        representations: vec![],
    };

    let config = Config::default();
    let result = generate(&app, &config);
    // Empty application generates empty code
    assert_eq!(result, "");
}

#[cfg(feature = "codegen")]
#[test]
fn test_config_client_trait_name() {
    let config = Config::default();
    assert_eq!(config.client_trait_name(), "wadl::blocking::Client");

    let async_config = Config {
        r#async: true,
        ..Default::default()
    };
    assert_eq!(async_config.client_trait_name(), "wadl::r#async::Client");
}

#[cfg(feature = "codegen")]
#[test]
fn test_generate_resource_name_clashes_with_resource_type() {
    let app = wadl::parse_string(
        r##"<?xml version="1.0"?>
<application xmlns="http://wadl.dev.java.net/2009/02">
  <resources base="https://example.com/">
    <resource path="things" type="#things"/>
  </resources>
  <resource_type id="things">
    <method name="GET" id="list"><request/><response/></method>
  </resource_type>
</application>"##,
    )
    .unwrap();
    let code = generate(&app, &Config::default());
    let structs = code
        .lines()
        .filter(|l| l.contains("struct "))
        .collect::<Vec<_>>();
    assert_eq!(
        structs,
        vec![
            "pub struct Things (reqwest::Url);",
            "pub struct ThingsResource (reqwest::Url);"
        ]
    );
}
