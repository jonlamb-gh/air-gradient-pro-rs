#![deny(warnings, clippy::all)]

fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");

    env_config::generate_env_config_constants();
}
