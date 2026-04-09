fn main() {
    // Cargo sets `TARGET` only for build scripts; `lib.rs` cannot see it via `option_env!` unless we forward it.
    println!(
        "cargo:rustc-env=AUDIO_HAXOR_TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("Cargo sets TARGET when running build.rs")
    );
    tauri_build::build();
}
