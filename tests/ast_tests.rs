use url::Url;
use wadl::ast::*;

#[test]
fn test_application_get_resource_type_by_id() {
    let app = Application {
        resources: vec![],
        resource_types: vec![
            ResourceType {
                id: "user".to_string(),
                query_type: mime::APPLICATION_JSON,
                methods: vec![],
                docs: vec![],
                subresources: vec![],
                params: vec![],
            },
            ResourceType {
                id: "admin".to_string(),
                query_type: mime::APPLICATION_JSON,
                methods: vec![],
                docs: vec![],
                subresources: vec![],
                params: vec![],
            },
        ],
        docs: vec![],
        grammars: vec![],
        representations: vec![],
    };

    // Test finding existing resource type
    assert!(app.get_resource_type_by_id("user").is_some());
    assert_eq!(app.get_resource_type_by_id("user").unwrap().id, "user");

    // Test not finding non-existent resource type
    assert!(app.get_resource_type_by_id("nonexistent").is_none());
}

#[test]
fn test_application_get_resource_type_by_href() {
    let app = Application {
        resources: vec![],
        resource_types: vec![ResourceType {
            id: "user".to_string(),
            query_type: mime::APPLICATION_JSON,
            methods: vec![],
            docs: vec![],
            subresources: vec![],
            params: vec![],
        }],
        docs: vec![],
        grammars: vec![],
        representations: vec![],
    };

    // Test with fragment
    let url_with_fragment = Url::parse("http://example.com#user").unwrap();
    assert!(app.get_resource_type_by_href(&url_with_fragment).is_some());

    // Test without fragment
    let url_without_fragment = Url::parse("http://example.com").unwrap();
    assert!(app
        .get_resource_type_by_href(&url_without_fragment)
        .is_none());

    // Test with non-matching fragment
    let url_wrong_fragment = Url::parse("http://example.com#nonexistent").unwrap();
    assert!(app.get_resource_type_by_href(&url_wrong_fragment).is_none());
}

#[test]
fn test_application_iter_resources_empty() {
    let app = Application {
        resources: vec![],
        resource_types: vec![],
        docs: vec![],
        grammars: vec![],
        representations: vec![],
    };

    let resources: Vec<_> = app.iter_resources().collect();
    assert_eq!(resources.len(), 0);
}

#[test]
fn test_application_get_resource_by_href() {
    let base_url = Url::parse("http://example.com/api/").unwrap();
    let resource = Resource {
        id: Some("users".to_string()),
        path: Some("users".to_string()),
        r#type: vec![],
        query_type: mime::APPLICATION_JSON,
        params: vec![],
        methods: vec![],
        subresources: vec![],
        docs: vec![],
    };

    let resources_group = Resources {
        base: Some(base_url.clone()),
        resources: vec![resource],
    };

    let app = Application {
        resources: vec![resources_group],
        resource_types: vec![],
        docs: vec![],
        grammars: vec![],
        representations: vec![],
    };

    let search_url = Url::parse("http://example.com/api/users").unwrap();
    let found = app.get_resource_by_href(&search_url);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id.as_ref().unwrap(), "users");

    let not_found_url = Url::parse("http://example.com/api/notfound").unwrap();
    assert!(app.get_resource_by_href(&not_found_url).is_none());
}

#[test]
fn test_options_empty() {
    let options = Options::new();
    assert!(options.is_empty());

    let keys: Vec<_> = options.keys().collect();
    assert_eq!(keys.len(), 0);

    let iter: Vec<_> = options.iter().collect();
    assert_eq!(iter.len(), 0);
}

#[test]
fn test_options_with_data() {
    let mut options = Options::new();
    options.insert("json".to_string(), Some(mime::APPLICATION_JSON));
    options.insert("xml".to_string(), None);

    assert!(!options.is_empty());

    let keys: Vec<_> = options.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"json"));
    assert!(keys.contains(&"xml"));

    let iter: Vec<_> = options.iter().collect();
    assert_eq!(iter.len(), 2);
}

#[test]
fn test_options_from_vec_string() {
    let vec = vec!["json".to_string(), "xml".to_string()];

    let options: Options = vec.into();
    assert!(!options.is_empty());
    assert_eq!(options.keys().count(), 2);
}

#[test]
fn test_options_from_vec_str() {
    let vec = vec!["json", "xml"];

    let options: Options = vec.into();
    assert!(!options.is_empty());
    assert_eq!(options.keys().count(), 2);
}

#[test]
fn test_resource_iter_referenced_types_empty() {
    let resource = Resource {
        id: None,
        path: None,
        r#type: vec![],
        query_type: mime::APPLICATION_JSON,
        params: vec![],
        methods: vec![],
        subresources: vec![],
        docs: vec![],
    };

    let types: Vec<_> = resource.iter_referenced_types().collect();
    assert_eq!(types.len(), 0);
}
