//! Build script for `rh-golden` to capture the engine build commit (BCK-002 A-70; SPR-027).

#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::collapsible_if,
    reason = "build script queries git commit and environment metadata for BCK-002 A-70"
)]

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RH_BUILD_COMMIT");
    if let Ok(commit) = std::env::var("RH_BUILD_COMMIT") {
        let trimmed = commit.trim();
        if !trimmed.is_empty() {
            println!("cargo:rustc-env=RH_BUILD_COMMIT={trimmed}");
            return;
        }
    }

    if let Ok(output) = Command::new("git")
        .args(["describe", "--always", "--dirty"])
        .output()
    {
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !commit.is_empty() {
                println!("cargo:rustc-env=RH_BUILD_COMMIT={commit}");
            }
        }
    }

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
    {
        if output.status.success() {
            let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let head_path = std::path::Path::new(&git_dir).join("HEAD");
            if head_path.exists() {
                println!("cargo:rerun-if-changed={}", head_path.display());
            }
        }
    }
}
