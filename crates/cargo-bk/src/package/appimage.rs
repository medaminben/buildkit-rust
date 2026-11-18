//! The `AppImage` format.

use super::{PackageCtx, copy_file, copy_tree, ensure_parent};
use crate::exec::{Cmd, Runner};
use crate::names::file_stem;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// The pinned appimagetool. `bootstrap/bootstrap.sh` carries the same values.
pub const APPIMAGETOOL_URL: &str =
    "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage";
pub const APPIMAGETOOL_SHA256: &str =
    "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0";

/// Builds `<out>/<Display-name>-<version>-x86_64.AppImage` from `stage`.
///
/// The `AppDir` is assembled in `<root>/build/appdir`; the whole stage goes to `usr/bin/`, so
/// files sit next to the executables as in the Windows install.
///
/// appimagetool comes from `PATH`, else it is downloaded to `<root>/build/tools` and verified.
///
/// # Errors
/// When a file is missing, a file operation fails, the tool cannot be
/// fetched, or appimagetool fails.
pub fn build_appimage(ctx: &PackageCtx, r: &dyn Runner) -> Result<PathBuf> {
    let PackageCtx {
        root,
        stage,
        out,
        meta,
        kebab,
        ..
    } = *ctx;
    let appdir = root.join("build").join("appdir");
    if appdir.exists() {
        fs::remove_dir_all(&appdir)
            .with_context(|| format!("cannot clear {}", appdir.display()))?;
    }
    let bin = appdir.join("usr/bin");
    fs::create_dir_all(&bin).with_context(|| format!("cannot create {}", bin.display()))?;
    copy_tree(stage, &bin)?;
    for app in &meta.apps {
        make_executable(&bin.join(app))?;
    }
    copy_file(
        &root.join(&meta.desktop),
        &appdir.join(format!("{kebab}.desktop")),
    )?;
    copy_file(&root.join(&meta.icon), &appdir.join(format!("{kebab}.png")))?;
    link_app_run(&appdir, &meta.main_app)?;

    let tool = if let Some(p) = r.which("appimagetool") {
        p
    } else {
        let p = root.join("build/tools/appimagetool-x86_64.AppImage");
        if !has_sha256(&p, APPIMAGETOOL_SHA256) {
            fetch_tool(r, &p, APPIMAGETOOL_URL, APPIMAGETOOL_SHA256)?;
        }
        make_executable(&p)?;
        p
    };

    fs::create_dir_all(out).with_context(|| format!("cannot create {}", out.display()))?;
    let name = file_stem(&meta.display_name);
    let image = out.join(format!("{name}-{}-x86_64.AppImage", ctx.version.full));
    let res = r.run(
        &Cmd::new(
            &tool.display().to_string(),
            &[&appdir.display().to_string(), &image.display().to_string()],
        )
        .cwd(root)
        .env(&[("APPIMAGE_EXTRACT_AND_RUN", "1"), ("ARCH", "x86_64")]),
    )?;
    if res.status != 0 {
        bail!("appimagetool failed: {}", res.stderr.trim());
    }
    Ok(image)
}

/// Downloads `url` to `dest` with curl and checks its SHA-256. A file that
/// does not match is deleted.
///
/// # Errors
/// When curl fails or the hash differs.
pub fn fetch_tool(r: &dyn Runner, dest: &Path, url: &str, sha256: &str) -> Result<()> {
    ensure_parent(dest)?;
    let res = r.run(&Cmd::new(
        "curl",
        &["-fsSL", "-o", &dest.display().to_string(), url],
    ))?;
    if res.status != 0 {
        bail!("download of {url} failed: {}", res.stderr.trim());
    }
    if !has_sha256(dest, sha256) {
        let _ = fs::remove_file(dest);
        bail!("SHA-256 of {url} does not match the pinned value; file deleted");
    }
    Ok(())
}

fn has_sha256(path: &Path, expected: &str) -> bool {
    fs::read(path)
        .is_ok_and(|data| format!("{:x}", Sha256::digest(&data)).eq_ignore_ascii_case(expected))
}

#[cfg(unix)]
fn make_executable(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("cannot set permissions on {}", p.display()))
}

#[cfg(not(unix))]
fn make_executable(_p: &Path) -> Result<()> {
    Ok(())
}

/// `AppRun` is a relative symlink to the main app.
#[cfg(unix)]
fn link_app_run(appdir: &Path, main_app: &str) -> Result<()> {
    std::os::unix::fs::symlink(format!("usr/bin/{main_app}"), appdir.join("AppRun"))
        .context("cannot create the AppRun symlink")
}

#[cfg(not(unix))]
fn link_app_run(_appdir: &Path, _main_app: &str) -> Result<()> {
    bail!("AppImages are built on Linux")
}
