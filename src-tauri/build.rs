use std::path::Path;
use std::process::Command;

fn git_first_line(repo_root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn main() {
    // Cargo sets `TARGET` only for build scripts; `lib.rs` cannot see it via `option_env!` unless we forward it.
    println!(
        "cargo:rustc-env=AUDIO_HAXOR_TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("Cargo sets TARGET when running build.rs")
    );

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap_or(manifest_dir);

    let full =
        git_first_line(repo_root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let short = if full == "unknown" || full.len() < 7 {
        full.clone()
    } else {
        full[..7].to_string()
    };
    let date = git_first_line(repo_root, &["log", "-1", "--format=%cI"]).unwrap_or_default();

    println!("cargo:rustc-env=AUDIO_HAXOR_GIT_SHA_FULL={full}");
    println!("cargo:rustc-env=AUDIO_HAXOR_GIT_SHA_SHORT={short}");
    println!("cargo:rustc-env=AUDIO_HAXOR_GIT_COMMIT_DATE={date}");

    let git_head = repo_root.join(".git/HEAD");
    if git_head.is_file() {
        println!("cargo:rerun-if-changed={}", git_head.display());
    }

    tauri_build::build();
}
