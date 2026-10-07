//! The files in `examples/` are the language spec. These tests keep the
//! compiler pointed at them; later steps add parse and check assertions here.

use std::path::{Path, PathBuf};

use magnetite_syntax::{SourceFile, discover};

fn examples_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn shop_example_has_all_seven_modules() {
    let modules: Vec<String> = discover(&examples_dir("shop"))
        .unwrap()
        .iter()
        .map(|path| SourceFile::load(path).unwrap().module_name().to_string())
        .collect();

    assert_eq!(
        modules,
        [
            "catalog",
            "checkout",
            "customers",
            "geo",
            "integrations",
            "orders",
            "shop"
        ]
    );
}
