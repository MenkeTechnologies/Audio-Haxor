use std::path::Path;
use std::process::Command;

fn read_package_json_version(repo_root: &Path) -> Option<String> {
    let pkg = repo_root.join("package.json");
    let contents = std::fs::read_to_string(&pkg).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
    json.get("version")?.as_str().map(|s| s.to_string())
}

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

    if let Some(ver) = read_package_json_version(repo_root) {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={ver}");
    }
    println!("cargo:rerun-if-changed=../package.json");

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

    // Windows MSVC + CI: use the official Tauri workaround for STATUS_ENTRYPOINT_NOT_FOUND.
    // The default tauri_build embeds a GUI subsystem manifest that crashes console test binaries.
    // Solution: skip the app manifest, then manually embed a console-compatible one.
    // See: https://github.com/orgs/tauri-apps/discussions/11179
    //      https://github.com/tauri-apps/tauri/pull/4383#issuecomment-1212221864
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_ci_test = std::env::var("TAURI_CI_TEST").is_ok();

    if is_ci_test && target_os == "windows" && target_env == "msvc" {
        tauri_build::try_build(
            tauri_build::Attributes::new()
                .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
        )
        .expect("tauri_build failed");
        embed_windows_manifest_for_tests();
    } else {
        tauri_build::build();
    }
}

fn embed_windows_manifest_for_tests() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = manifest_dir.join("windows-test-manifest.xml");
    if !manifest.exists() {
        return;
    }
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest.to_str().unwrap()
    );
    println!("cargo:rustc-link-arg=/WX");
}
