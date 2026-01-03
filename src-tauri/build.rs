fn main() {
    // Install git hooks using cargo-husky if available
    if std::env::var("CARGO_HUSKY").is_ok() {
        println!("cargo:rerun-if-env-changed=CARGO_HUSKY");
    }

    tauri_build::build()
}
