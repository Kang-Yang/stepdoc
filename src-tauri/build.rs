use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    #[cfg(windows)]
    ensure_directml_dll();

    tauri_build::build();
}

#[cfg(windows)]
fn ensure_directml_dll() {
    let Some(release_dir) = target_release_dir() else {
        return;
    };

    let dest = release_dir.join("DirectML.dll");
    if dest.exists() {
        println!("cargo:rerun-if-changed={}", dest.display());
        return;
    }

    if let Some(source) = find_directml_dll() {
        if fs::copy(&source, &dest).is_ok() {
            println!("cargo:warning=Copied DirectML.dll to {}", dest.display());
            println!("cargo:rerun-if-changed={}", source.display());
            return;
        }
    }

    println!(
        "cargo:warning=DirectML.dll not found; run a full `cargo build --release` before `tauri build`"
    );
}

#[cfg(windows)]
fn target_release_dir() -> Option<PathBuf> {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR")?);
    // OUT_DIR = .../target/<profile>/build/<pkg>/out
    let profile_dir = out_dir.ancestors().nth(3)?;
    Some(profile_dir.to_path_buf())
}

#[cfg(windows)]
fn find_directml_dll() -> Option<PathBuf> {
    if let Some(release_dir) = target_release_dir() {
        let candidate = release_dir.join("DirectML.dll");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let local_app = std::env::var_os("LOCALAPPDATA")?;
    let ort_cache = PathBuf::from(local_app).join("ort.pyke.io").join("dfbin");
    find_file_named(&ort_cache, "DirectML.dll")
}

#[cfg(windows)]
fn find_file_named(root: &Path, name: &str) -> Option<PathBuf> {
    if !root.is_dir() {
        return None;
    }

    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_named(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }

    None
}
