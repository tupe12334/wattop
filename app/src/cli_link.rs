//! Links the `wattop` CLI bundled in the app into ~/.local/bin.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// The `wattop` binary that the bundle ships next to the app executable.
pub fn bundled_cli() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()?
        .join("wattop")
        .canonicalize()
        .ok()
}

/// Symlinks ~/.local/bin/wattop to the bundled CLI. Returns the text for the menu info line.
pub fn install() -> String {
    let Some(src) = bundled_cli() else {
        return "CLI is not bundled in this build".into();
    };
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return "HOME is not set".into();
    };
    let bin = home.join(".local/bin");
    let dest = bin.join("wattop");
    let linked = fs::create_dir_all(&bin).and_then(|()| {
        let _ = fs::remove_file(&dest);
        symlink(&src, &dest)
    });
    match linked {
        Err(e) => format!("Link failed: {e}"),
        Ok(()) if on_path(&bin) => "CLI linked at ~/.local/bin/wattop".into(),
        Ok(()) => "CLI linked; ~/.local/bin is not on PATH".into(),
    }
}

fn on_path(dir: &Path) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d == dir))
}
