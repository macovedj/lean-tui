use std::{env, fs, path::PathBuf};

/// Pinned commit hash for lean-dag protocol schema.
const LEAN_DAG_COMMIT: &str = "84a98f0ffd89f5c9a10d5a19db1e89c65d7714fb";

fn main() {
    let schema_content = fetch_schema();

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

fn fetch_schema() -> String {
    // Allow override via environment variable for local development
    if let Ok(path) = env::var("LEAN_DAG_SCHEMA") {
        let path = PathBuf::from(path);
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
            return fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read schema at {}: {e}", path.display()));
        }
    }

    // Fetch from GitHub
    let url = format!(
        "https://raw.githubusercontent.com/wvhulle/lean-dag/{LEAN_DAG_COMMIT}/protocol-schema.json"
    );

    ureq::get(&url)
        .call()
        .unwrap_or_else(|e| panic!("Failed to fetch schema from {url}: {e}"))
        .into_string()
        .unwrap_or_else(|e| panic!("Failed to read schema response: {e}"))
}
