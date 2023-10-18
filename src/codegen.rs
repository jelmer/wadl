use crate::ast::*;
use proc_macro2::TokenStream;
use quote::quote;
use url::Url;

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
            if !result.is_empty() && result.chars().last().unwrap() != '_' {
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

fn generate_representation(input: &Representation) -> Vec<String> {
    let mut lines = vec![];
    for doc in &input.docs {
        lines.extend(generate_doc(doc));
    }

    lines.extend(generate_representation_struct(input));
    lines
}

fn generate_representation_struct(input: &Representation) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let name = input.id.as_ref().unwrap().as_str();
    let name = name.strip_suffix("-json").unwrap_or(name);
    let name = camel_case_name(name);

    lines.push(
        "#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]\n"
            .to_string(),
    );
    lines.push(format!("pub struct {} {{\n", name));

    for param in &input.params {
        let param_name = snake_case_name(param.name.as_str());

        assert!(param.id.is_none());
        assert!(param.fixed.is_none());

        let mut param_type = match &param.r#type {
            TypeRef::Simple(name) => match name.as_str() {
                "xsd:date" => "chrono::NaiveDate".to_string(),
                "xsd:dateTime" => "chrono::NaiveDateTime".to_string(),
                "xsd:duration" => "chrono::Duration".to_string(),
                "xsd:time" => "chrono::NaiveTime".to_string(),
                "string" => "String".to_string(),
                "binary" => "Vec<u8>".to_string(),
                u => panic!("Unknown type: {}", u),
            },
            TypeRef::EmptyLink => {
                // This would be a reference to the representation itself
                "url::Url".to_string()
            }
            TypeRef::ResourceTypeId(_) | TypeRef::ResourceTypeLink(_) => "url::Url".to_string(),
            TypeRef::Options(options) => {
                // TODO: define an enum for this
                "String".to_string()
            }
            TypeRef::NoType => match param.name.as_str() {
                "http_etag" => "String",
                "description" => "String",
                "scopes" => "Vec<String>",
                "start" | "total_size" => "usize",
                "entries" => "Vec<serde_json::Value>",
                "component_name" => "String",
                "pocket" => "String",
                "title" => "String",
                "authorized_size" => "usize",
                "display_name" | "displayname" => "String",
                "external_dependencies" => "String",
                "name" => "String",
                "private" => "bool",
                "publish" => "bool",
                "reference" => "String",
                "relative_build_score" => "f64",
                "require_virtualization" => "bool",
                "active" => "bool",
                "added_lines_count" => "usize",
                "address" => "String",
                "advertise_by_hash" => "bool",
                "allow_internet" => "bool",
                "allowspoilt" => "bool",
                "architecture_specific" => "bool",
                "architecture_tag" => "String",
                "arch_tag" => "String",
                "as_quoted_email" => "String",
                "auto_build" => "bool",
                "auto_build_channels" => "Vec<String>",
                "backports_not_automatic" => "bool",
                "base_url" => "url::Url",
                "base_url_aliases" => "Vec<url::Url>",
                "base_version" => "String",
                "binary_package_name" => "String",
                "binary_package_version" => "String",
                "body_text" => "String",
                "bug_reported_acknowledgement" => "bool",
                "bug_reporting_guidelines" => "String",
                "bug_target_display_name" => "String",
                "bug_target_name" => "String",
                "build_channels" => "Vec<String>",
                "build_daily" => "bool",
                "build_log_url" => "url::Url",
                "build_path" => "String",
                "build_snap_channels" => "Vec<String>",
                "build_source_tarball" => "url::Url",
                "bzr_identity" => "String",
                "can_be_cancelled" => "bool",
                "can_be_rescored" => "bool",
                "can_be_retried" => "bool",
                "can_expire" => "bool",
                "can_infer_distro_series" => "bool",
                "can_upload_to_store" => "bool",
                "changelog" => "String",
                "changesfile_url" => "url::Url",
                "changeslist" => "String",
                "channels" => "Vec<String>",
                "chroot_url" => "url::Url",
                "code" => "String",
                "code_name" => "String",
                "comment" => "String",
                "commercial_subscription_is_due" => "bool",
                "commit_message" => "String",
                "commit_sha1" => "String",
                "component_names" => "Vec<String>",
                "conflicts" => "Vec<String>",
                "contact_details" => "String",
                "content" => "String",
                "count" => "usize",
                "country_dns_mirror" => "String",
                "custom_file_urls" => "Vec<url::Url>",
                "cvs_module" => "String",
                "cvs_root" => "String",
                "deb_version_template" => "String",
                "default_branch" => "String",
                "default_membership_period" => "chrono::Duration",
                "default_renewal_period" => "chrono::Duration",
                "delivery_url" => "url::Url",
                "dependencies" => "Vec<String>",
                "development_series_alias" => "String",
                "diff_lines_count" => "usize",
                "diffstat" => "String",
                "display_arches" => "Vec<String>",
                "display_version" => "String",
                "distroseries" => "String",
                "distro_series_name" => "String",
                "domain_name" => "String",
                "download_url" => "url::Url",
                "email" => "String",
                "enabled" => "bool",
                "english_name" => "String",
                "error_message" => "String",
                "error_output" => "String",
                "event_type" => "String",
                "event_types" => "Vec<String>",
                "explicit" => "bool",
                "explicitly_private" => "bool",
                "exported_in_languagepacks" => "bool",
                "failnotes" => "String",
                "failure_count" => "usize",
                "features" => "Vec<String>",
                "find_all_tags" => "bool",
                "fingerprint" => "String",
                "freshmeat_project" => "String",
                "ftp_base_url" => "url::Url",
                "fullseriesname" => "String",
                "git_https_url" => "url::Url",
                "git_identity" => "String",
                "git_path" => "String",
                "git_ref_pattern" => "String",
                "git_refs" => "Vec<String>",
                "git_repository_url" => "url::Url",
                "git_ssh_url" => "url::Url",
                "has_lp_plugin" => "bool",
                "heat" => "f64",
                "hide_email_addresses" => "bool",
                "homepage_content" => "String",
                "homepage_url" => "url::Url",
                "http_base_url" => "url::Url",
                "https_base_url" => "url::Url",
                "id" => "i64",
                "importances" => "Vec<String>",
                "include_long_descriptions" => "bool",
                "index_compressors" => "Vec<String>",
                "information_types" => "Vec<String>",
                "is_active" => "bool",
                "is_complete" => "bool",
                "is_default" => "bool",
                "is_development_focus" => "bool",
                "is_nominated_arch_indep" => "bool",
                "iso3166code2" => "String",
                "iso3166code3" => "String",
                "is_pending" => "bool",
                "is_permitted" => "bool",
                "is_probationary" => "bool",
                "is_stale" => "bool",
                "is_team" => "bool",
                "is_ubuntu_coc_signer" => "bool",
                "is_valid" => "bool",
                "is_visible" => "bool",
                "jabberid" => "String",
                "karma" => "i64",
                "keep_binary_files_days" => "usize",
                "keyid" => "String",
                "keytext" => "String",
                "landmarks" => "Vec<String>",
                "language_count" => "usize",
                "language_pack_full_export_requested" => "bool",
                "last_change_comment" => "String",
                "last_scanned_id" => "i64",
                "latest_published_component_name" => "String",
                "latitude" => "f64",
                "license_approved" => "bool",
                "license_info" => "String",
                "licenses" => "Vec<String>",
                "longitude" => "f64",
                "loose_object_count" => "usize",
                "manual" => "String",
                "merged_revision_id" => "String",
                "merged_revno" => "i64",
                "message" => "String",
                "message_body" => "String",
                "message_count" => "usize",
                "metadata" => "String",
                "metadata_override" => "String",
                "mirror_status_message" => "String",
                "network" => "String",
                "newvalue" => "String",
                "nickname" => "String",
                "number_of_duplicates" => "usize",
                "official" => "bool",
                "official_answers" => "bool",
                "official_blueprints" => "bool",
                "official_bugs" => "bool",
                "official_bug_tags" => "bool",
                "official_candidate" => "bool",
                "official_codehosting" => "bool",
                "official_packages" => "bool",
                "oldvalue" => "String",
                "open_resources" => "bool",
                "other_users_affected_count_with_dupes" => "usize",
                "owner_default" => "bool",
                "package_count" => "usize",
                "package_diff_url" => "url::Url",
                "package_set_name" => "String",
                "pack_count" => "usize",
                "parent_package_diff_url" => "url::Url",
                "parent_source_version" => "String",
                "path" => "String",
                "payload" => "String",
                "pending" => "bool",
                "permission" => "String",
                "phased_update_percentage" => "usize",
                "plural_expression" => "String",
                "plural_forms" => "String",
                "prerequisite_git_path" => "String",
                "prerequisite_revision_id" => "String",
                "priority" => "String",
                "priority_name" => "String",
                "private_bugs" => "bool",
                "programming_language" => "String",
                "project_reviewed" => "bool",
                "properties" => "Vec<String>",
                "proposed_not_automatic" => "bool",
                "proposition" => "String",
                "publish_by_hash" => "bool",
                "qualifies_for_free_hosting" => "bool",
                "recipe_text" => "String",
                "redirect_default_traversal" => "String",
                "redirect_release_uploads" => "bool",
                "release_finder_url_pattern" => "String",
                "release_notes" => "String",
                "remote_bug" => "String",
                "remote_importance" => "String",
                "remote_product" => "String",
                "remote_status" => "String",
                "removal_comment" => "String",
                "removed_lines_count" => "usize",
                "require_virtualized" => "bool",
                "restricted_resources" => "bool",
                "results" => "Vec<String>",
                "result_summary" => "String",
                "reviewed" => "bool",
                "reviewed_revid" => "String",
                "reviewer_whiteboard" => "String",
                "review_type" => "String",
                "revision_count" => "usize",
                "revision_id" => "String",
                "rsync_base_url" => "url::Url",
                "score" => "f64",
                "screenshots_url" => "url::Url",
                "section_name" => "String",
                "security_contact" => "String",
                "security_related" => "bool",
                "sequence" => "usize",
                "signing_key_fingerprint" => "String",
                "sourceforge_project" => "String",
                "source_git_path" => "String",
                "source_package_name" => "String",
                "sourcepackagename" => "String",
                "source_package_version" => "String",
                "source_revision_id" => "String",
                "source_version" => "String",
                "stages" => "Vec<String>",
                "stale" => "bool",
                "statuses" => "Vec<String>",
                "store_channels" => "Vec<String>",
                "store_name" => "String",
                "store_upload" => "String",
                "store_upload_error_message" => "String",
                "store_upload_error_messages" => "Vec<String>",
                "store_upload_revision" => "String",
                "store_upload_url" => "url::Url",
                "subject" => "String",
                "successful" => "bool",
                "suite_names" => "Vec<String>",
                "summary" => "String",
                "supported" => "bool",
                "supports_mirrors" => "bool",
                "supports_ppas" => "bool",
                "supports_virtualized" => "bool",
                "suppress_subscription_notifications" => "bool",
                "tags" => "Vec<String>",
                "target_architectures" => "Vec<String>",
                "target_default" => "bool",
                "target_git_path" => "String",
                "target_revision_id" => "String",
                "team_description" => "String",
                "time_zone" => "String",
                "token" => "String",
                "translation_domain" => "String",
                "translators_count" => "usize",
                "unique_key" => "String",
                "unique_name" => "String",
                "upload_log_url" => "url::Url",
                "uri" => "url::Url",
                "url" => "url::Url",
                "usable_distro_series" => "bool",
                "users_affected_count" => "usize",
                "users_affected_count_with_dupes" => "usize",
                "users_unaffected_count" => "usize",
                "version" => "String",
                "virtualized" => "bool",
                "visible" => "bool",
                "vm_host" => "String",
                "vote_tag" => "String",
                "whatchanged" => "String",
                "whiteboard" => "String",
                "wiki" => "String",
                "wikiname" => "String",
                "wiki_url" => "url::Url",
                n => {
                    println!("No type for parameter: {}", n);
                    "serde_json::Value"
                }
            }
            .to_string(),
        };

        if param.repeating {
            param_type = format!("Vec<{}>", param_type);
        }

        if !param.required {
            param_type = format!("Option<{}>", param_type);
        }

        lines.push(format!("    pub {}: {},\n", param_name, param_type));
    }

    lines.push("}\n".to_string());
    lines.push("\n".to_string());

    lines
}

pub fn generate(app: &Application) -> String {
    let mut lines = vec![];

    for doc in &app.docs {
        lines.extend(generate_doc(doc));
    }

    for representation in &app.representations {
        lines.extend(generate_representation(representation));
    }

    lines.concat()
}
