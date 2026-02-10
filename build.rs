use std::{env, fs, path::PathBuf};

const LEAN_PRISM_COMMIT: &str = "af86afc";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let schema_json = env::var("LEAN_DAG_SCHEMA")
        .map(PathBuf::from)
        .ok()
        .filter(|p| p.exists())
        .map_or_else(
            || {
                let url = format!(
                    "https://codeberg.org/wvhulle/lean-prism/raw/commit/{LEAN_PRISM_COMMIT}/protocol-schema.json"
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
