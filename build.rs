use std::{env, fs, path::PathBuf};

const LEAN_DAG_COMMIT: &str = "b9a1143ca9342ad4c732132e8a01711af65531a6";

fn main() {
    let schema_json = env::var("LEAN_DAG_SCHEMA")
        .map(PathBuf::from)
        .ok()
        .filter(|p| p.exists())
        .map_or_else(
            || {
                let url = format!(
                    "https://codeberg.org/wvhulle/lean-dag/raw/commit/{LEAN_DAG_COMMIT}/protocol-schema.json"
                );
                ureq::get(&url).call().expect("fetch schema").into_string().expect("read response")
            },
            |p| {
                println!("cargo:rerun-if-changed={}", p.display());
                fs::read_to_string(&p).expect("read local schema")
            },
        );

    let mut type_space = typify::TypeSpace::default();
    type_space
        .add_root_schema(serde_json::from_str(&schema_json).expect("parse schema"))
        .expect("add schema");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let code = prettyplease::unparse(&syn::parse2(type_space.to_stream()).unwrap());

    fs::write(out_dir.join("protocol_types.rs"), code).unwrap();
}
