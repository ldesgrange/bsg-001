#[path = "support/architecture.rs"]
mod architecture;

use architecture::{Layer, check_source};

#[test]
fn rejects_forbidden_paths_and_infrastructure() {
    for (layer, source) in [
        (Layer::Domain, "use crate::{application::Thing as T};"),
        (Layer::Domain, "use super::super::adapters::Thing;"),
        (Layer::Domain, "fn f() { btleplug::api::foo(); }"),
        (Layer::Domain, "extern crate btleplug as bluetooth;"),
        (Layer::Domain, "use tokio::time::sleep;"),
        (Layer::Application, "use crate::adapters::Thing;"),
        (Layer::Application, "use clap::Parser;"),
        (Layer::Application, "use tokio::time::sleep;"),
    ] {
        assert!(!check_source(layer, source).unwrap().is_empty(), "missed {source}");
    }
}

#[test]
fn accepts_inward_paths_and_ignores_comments_and_strings() {
    let source = r#"
        use crate::domain::DeviceInfo;
        // crate::adapters::Forbidden
        const EXAMPLE: &str = "btleplug::Bad";
    "#;
    assert!(check_source(Layer::Application, source).unwrap().is_empty());
}

#[test]
fn invalid_rust_is_an_error() {
    assert!(check_source(Layer::Domain, "use {").is_err());
}

#[test]
fn missing_source_directory_is_an_error() {
    assert!(architecture::check_tree(std::path::Path::new("nonexistent-architecture-root")).is_err());
}

#[test]
fn repository_layers_obey_boundaries() {
    architecture::check_tree(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").as_path()).unwrap();
}
