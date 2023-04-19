#![deny(warnings, clippy::all)]

fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");

    env_config::generate_env_config_constants();

    // TODO - can't watch this since agp-linker changes it at link-time
    // println!("cargo:rerun-if-changed=memory.x");
}
