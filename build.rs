use std::{env, fs, path::PathBuf};

fn main() {
    let schema_path = find_schema_path();

    println!("cargo:rerun-if-changed={}", schema_path.display());

    let schema_content = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("Failed to read schema at {}: {e}", schema_path.display()));

    let schema: serde_json::Value = serde_json::from_str(&schema_content)
        .unwrap_or_else(|e| panic!("Failed to parse schema JSON: {e}"));

    let mut type_space = typify::TypeSpace::default();

    type_space
        .add_root_schema(serde_json::from_value(schema).unwrap())
        .unwrap();

    let contents = prettyplease::unparse(&syn::parse2(type_space.to_stream()).unwrap());

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("protocol_types.rs"), contents).unwrap();
}

fn find_schema_path() -> PathBuf {
    // Try relative path from lean-tui to sibling lean-dag directory
    let sibling_path = PathBuf::from("../lean-dag/protocol-schema.json");
    if sibling_path.exists() {
        return sibling_path;
    }

    // Try environment variable
    if let Ok(path) = env::var("LEAN_DAG_SCHEMA") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }

    // Try home directory
    if let Some(home) = dirs::home_dir() {
        let home_path = home.join("Code/lean-dag/protocol-schema.json");
        if home_path.exists() {
            return home_path;
        }
    }

    panic!(
        "Could not find protocol-schema.json. Set LEAN_DAG_SCHEMA env var or place lean-dag as \
         sibling directory."
    );
}
