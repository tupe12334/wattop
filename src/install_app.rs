//! `wattop install-app` and `wattop uninstall-app`: fetch and install the menu-bar app that matches this CLI.

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP: &str = "Wattop.app";
/// Test hook: install into this folder instead. Skips quitting the running app.
const TEST_DIR_VAR: &str = "WATTOP_INSTALL_DIR";

/// Command line. Bare `wattop` (no arguments) runs the dashboard and never reaches clap.
#[derive(Parser, Debug)]
#[command(
    name = "wattop",
    version,
    about = "Terminal power dashboard. Run `wattop` with no arguments for the dashboard (press q to quit).",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Install the menu-bar app for this CLI version
    InstallApp {
        /// Release to install; a leading `v` is dropped
        #[arg(long, value_name = "X", default_value = VERSION, value_parser = parse_version)]
        version: String,
        /// Do not open the app after installing
        #[arg(long)]
        no_open: bool,
    },
    /// Remove the menu-bar app
    UninstallApp,
}

#[derive(Debug, PartialEq)]
pub struct InstallOpts {
    pub version: String,
    pub open: bool,
}

#[derive(Debug, PartialEq)]
pub enum InstallDir {
    System,
    User,
}

/// Download URL of the Apple Silicon app zip for a release tag `wattop-v<version>`.
pub fn asset_url(version: &str) -> String {
    format!(
        "https://github.com/tupe12334/wattop/releases/download/wattop-v{version}/Wattop-wattop-v{version}-aarch64.app.zip"
    )
}

/// /Applications when writable, else ~/Applications, else nowhere.
pub fn pick_install_dir(system_writable: bool, user_writable: bool) -> Option<InstallDir> {
    if system_writable {
        Some(InstallDir::System)
    } else if user_writable {
        Some(InstallDir::User)
    } else {
        None
    }
}

/// Accepts `[v]X` where X is alphanumerics and `._-+`; returns X without the `v`.
fn parse_version(v: &str) -> Result<String, String> {
    let v = v.trim_start_matches('v');
    if v.is_empty()
        || !v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c))
    {
        return Err(format!("bad version: {v}"));
    }
    Ok(v.to_string())
}

/// True when the menu-bar app sits in either install folder.
pub fn app_installed() -> bool {
    system_app().exists() || user_app().is_some_and(|p| p.exists())
}

/// Runs the subcommand. Returns the process exit code (clap: 0 for help/version, 2 for usage errors).
pub fn dispatch(args: &[String]) -> i32 {
    let cli =
        match Cli::try_parse_from(std::iter::once("wattop").chain(args.iter().map(String::as_str)))
        {
            Ok(cli) => cli,
            Err(e) => {
                let _ = e.print();
                return e.exit_code();
            }
        };
    let result = match cli.cmd {
        Some(Cmd::InstallApp { version, no_open }) => install(&InstallOpts {
            version,
            open: !no_open,
        }),
        Some(Cmd::UninstallApp) => uninstall(),
        None => {
            let _ = Cli::command().print_help();
            return 2;
        }
    };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e:#}");
            1
        }
    }
}

fn install(opts: &InstallOpts) -> Result<()> {
    if !cfg!(target_arch = "aarch64") {
        bail!(
            "the menu-bar app is built for Apple Silicon (aarch64) only; this CLI is {}",
            std::env::consts::ARCH
        );
    }
    let url = asset_url(&opts.version);
    let work = std::env::temp_dir().join(format!("wattop-install-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work)?;
    let zip = work.join("Wattop.app.zip");

    println!("1/4 downloading {url}");
    run(Command::new("curl")
        .args(["-fL", "--progress-bar", "-o"])
        .arg(&zip)
        .arg(&url))
    .with_context(|| format!("download failed; is there a release for v{}?", opts.version))?;

    println!("2/4 extracting");
    run(Command::new("ditto")
        .arg("-x")
        .arg("-k")
        .arg(&zip)
        .arg(&work))?;
    let staged = work.join(APP);
    if !staged.exists() {
        bail!("the zip has no {APP}");
    }

    let target = target_app()?;
    println!("3/4 installing to {}", target.display());
    quit_app();
    if target.exists() {
        fs::remove_dir_all(&target)
            .with_context(|| format!("cannot remove {}", target.display()))?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    run(Command::new("ditto").arg(&staged).arg(&target))?;
    let _ = Command::new("xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&target)
        .status();
    let _ = fs::remove_dir_all(&work);

    if opts.open {
        println!("4/4 opening");
        run(Command::new("open").arg(&target))?;
    }
    println!("installed {}", target.display());
    println!(
        "Use the menu bar item, then Install command-line tool to link wattop into ~/.local/bin."
    );
    Ok(())
}

fn uninstall() -> Result<()> {
    quit_app();
    let mut removed = false;
    for app in [Some(system_app()), user_app()].into_iter().flatten() {
        if app.exists() {
            fs::remove_dir_all(&app).with_context(|| format!("cannot remove {}", app.display()))?;
            println!("removed {}", app.display());
            removed = true;
        }
    }
    if !removed {
        println!("{APP} is not installed");
    }
    Ok(())
}

/// Where install writes the app. The test hook wins; else the first writable folder.
fn target_app() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os(TEST_DIR_VAR) {
        return Ok(PathBuf::from(dir).join(APP));
    }
    let system = system_app();
    let system_ok = writable(system.parent().unwrap_or(Path::new("/Applications")));
    let user = user_app().context("HOME is not set")?;
    let user_ok = writable(user.parent().unwrap_or(Path::new(".")));
    match pick_install_dir(system_ok, user_ok) {
        Some(InstallDir::System) => Ok(system),
        Some(InstallDir::User) => Ok(user),
        None => bail!("neither /Applications nor ~/Applications is writable"),
    }
}

/// Quits the running app. Skipped under the test hook so a real install is not touched.
fn quit_app() {
    if std::env::var_os(TEST_DIR_VAR).is_none() {
        let _ = Command::new("osascript")
            .args(["-e", r#"quit app "Wattop""#])
            .status();
    }
}

/// Creates the folder if needed, then proves it accepts a file.
fn writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".wattop-write-probe");
    let ok = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

fn system_app() -> PathBuf {
    PathBuf::from("/Applications").join(APP)
}

fn user_app() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Applications").join(APP))
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("cannot start {:?}", cmd.get_program()))?;
    if !status.success() {
        bail!("{:?} exited with {status}", cmd.get_program());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_url_matches_release_zip_name() {
        assert_eq!(
            asset_url("0.2.0"),
            "https://github.com/tupe12334/wattop/releases/download/wattop-v0.2.0/Wattop-wattop-v0.2.0-aarch64.app.zip"
        );
    }

    #[test]
    fn install_dir_prefers_system_then_user() {
        assert_eq!(pick_install_dir(true, true), Some(InstallDir::System));
        assert_eq!(pick_install_dir(true, false), Some(InstallDir::System));
        assert_eq!(pick_install_dir(false, true), Some(InstallDir::User));
        assert_eq!(pick_install_dir(false, false), None);
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("wattop").chain(args.iter().copied()))
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_install_flags() {
        let c = parse(&["install-app", "--version", "v0.1.0", "--no-open"]).unwrap();
        assert!(
            matches!(c.cmd, Some(Cmd::InstallApp { ref version, no_open: true }) if version == "0.1.0")
        );
        let c = parse(&["install-app", "--version=0.1.0"]).unwrap();
        assert!(
            matches!(c.cmd, Some(Cmd::InstallApp { ref version, no_open: false }) if version == "0.1.0")
        );
        let c = parse(&["install-app"]).unwrap();
        assert!(
            matches!(c.cmd, Some(Cmd::InstallApp { ref version, no_open: false }) if version == VERSION)
        );
        assert!(matches!(
            parse(&["uninstall-app"]).unwrap().cmd,
            Some(Cmd::UninstallApp)
        ));
    }

    #[test]
    fn version_and_help_exit_zero() {
        let e = parse(&["--version"]).unwrap_err();
        assert_eq!(e.kind(), clap::error::ErrorKind::DisplayVersion);
        assert_eq!(e.to_string().trim(), format!("wattop {VERSION}"));
        assert_eq!(dispatch(&["--version".to_string()]), 0);
        assert_eq!(dispatch(&["-V".to_string()]), 0);
        assert_eq!(
            dispatch(&["install-app".to_string(), "--help".to_string()]),
            0
        );
    }

    #[test]
    fn rejects_bad_args() {
        assert!(parse(&["install-app", "--version"]).is_err());
        assert!(parse(&["install-app", "--version", "1.0/../x"]).is_err());
        assert!(parse(&["install-app", "--bogus"]).is_err());
        assert_eq!(dispatch(&["bogus".to_string()]), 2);
    }
}
