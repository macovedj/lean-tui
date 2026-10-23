use std::{env, fs, path::PathBuf};

const SCHEMA_PATH_VARIABLE: &str = "LEAN_DAG_SCHEMA";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={SCHEMA_PATH_VARIABLE}");

    let schema_path = PathBuf::from(env::var(SCHEMA_PATH_VARIABLE).unwrap_or_else(|_| {
        panic!(
            "{SCHEMA_PATH_VARIABLE} is not set: point it at Lean-Prism's protocol-schema.json. \
             `nix develop` and `nix build` set it from the `lean-prism` flake input."
        )
    }));
    println!("cargo:rerun-if-changed={}", schema_path.display());
    let schema_json = fs::read_to_string(&schema_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", schema_path.display()));

    let mut type_space = typify::TypeSpace::default();
    type_space
        .add_root_schema(serde_json::from_str(&schema_json).expect("parse schema"))
        .expect("add schema");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let code = prettyplease::unparse(&syn::parse2(type_space.to_stream()).unwrap());

    fs::write(out_dir.join("protocol_types.rs"), code).unwrap();
}
