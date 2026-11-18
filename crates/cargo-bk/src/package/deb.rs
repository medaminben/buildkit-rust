//! The Debian package format.

use super::{PackageCtx, copy_file, copy_tree, ensure_parent};
use crate::exec::{Cmd, Runner};
use crate::workspace::PackageMeta;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Maps a product version to a Debian one: the pre-release dash becomes `~`,
/// so `1.2.3~beta.4` sorts before `1.2.3`.
#[must_use]
pub fn deb_version(full: &str) -> String {
    full.replacen('-', "~", 1)
}

/// The `DEBIAN/control` text.
#[must_use]
pub fn deb_control(
    meta: &PackageMeta,
    kebab: &str,
    maintainer: &str,
    version: &str,
    installed_size_kib: u64,
) -> String {
    let mut lines = vec![
        format!("Package: {kebab}"),
        format!("Version: {version}"),
        "Architecture: amd64".to_string(),
        format!("Maintainer: {maintainer}"),
        format!("Installed-Size: {installed_size_kib}"),
    ];
    if !meta.system_deps_apt.is_empty() {
        lines.push(format!("Depends: {}", meta.system_deps_apt.join(", ")));
    }
    lines.push(format!("Description: {}", meta.display_name));
    lines.join("\n") + "\n"
}

/// Builds `<out>/<kebab>_<version>_amd64.deb` from `stage`.
///
/// `meta.desktop` and `meta.icon` are read relative to the workspace `root`. The package tree
/// is assembled in `<root>/build/deb-root`.
///
/// The whole stage goes to `/usr/lib/<kebab>/`, so files sit next to the executables as in the
/// Windows install; `/usr/bin/<app>` are relative symlinks to them.
///
/// # Errors
/// When a file is missing, a file operation fails, or `dpkg-deb` fails.
pub fn build_deb(ctx: &PackageCtx, r: &dyn Runner) -> Result<PathBuf> {
    let PackageCtx {
        root,
        stage,
        out,
        meta,
        kebab,
        maintainer,
        ..
    } = *ctx;
    let version = deb_version(&ctx.version.full);
    let pkg = root.join("build").join("deb-root");
    if pkg.exists() {
        fs::remove_dir_all(&pkg).with_context(|| format!("cannot clear {}", pkg.display()))?;
    }
    let lib = pkg.join("usr/lib").join(kebab);
    copy_tree(stage, &lib)?;
    let bin = pkg.join("usr/bin");
    fs::create_dir_all(&bin).with_context(|| format!("cannot create {}", bin.display()))?;
    for app in &meta.apps {
        symlink(&format!("../lib/{kebab}/{app}"), &bin.join(app))?;
    }
    copy_file(
        &root.join(&meta.desktop),
        &pkg.join(format!("usr/share/applications/{kebab}.desktop")),
    )?;
    copy_file(
        &root.join(&meta.icon),
        &pkg.join(format!("usr/share/icons/hicolor/256x256/apps/{kebab}.png")),
    )?;
    let mut texts = Vec::new();
    for name in ["LICENSE", "THIRD_PARTY_NOTICES.txt"] {
        let p = stage.join(name);
        if p.is_file() {
            texts.push(fs::read_to_string(&p)?);
        }
    }
    let last = texts.pop().unwrap_or_default();
    let mut copyright: Vec<&str> = texts.iter().map(|t| t.trim_end()).collect();
    copyright.push(&last);
    let copyright = copyright.join("\n\n");
    let doc = pkg.join(format!("usr/share/doc/{kebab}/copyright"));
    write(&doc, copyright.as_bytes())?;

    let size = installed_size_kib(&pkg);
    write(
        &pkg.join("DEBIAN/control"),
        deb_control(meta, kebab, maintainer, &version, size).as_bytes(),
    )?;
    set_modes(&pkg, &lib, &meta.apps)?;

    fs::create_dir_all(out).with_context(|| format!("cannot create {}", out.display()))?;
    let deb = out.join(format!("{kebab}_{version}_amd64.deb"));
    let res = r.run(
        &Cmd::new(
            "dpkg-deb",
            &[
                "--root-owner-group",
                "--build",
                &pkg.display().to_string(),
                &deb.display().to_string(),
            ],
        )
        .cwd(root),
    )?;
    if res.status != 0 {
        bail!("dpkg-deb failed: {}", res.stderr.trim());
    }
    Ok(deb)
}

fn write(dest: &Path, data: &[u8]) -> Result<()> {
    ensure_parent(dest)?;
    fs::write(dest, data).with_context(|| format!("cannot write {}", dest.display()))
}

/// The size of all files under `pkg`, in KiB, rounded up.
fn installed_size_kib(pkg: &Path) -> u64 {
    let bytes: u64 = WalkDir::new(pkg)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
    bytes.div_ceil(1024)
}

#[cfg(unix)]
fn symlink(target: &str, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)
        .with_context(|| format!("cannot create the symlink {}", link.display()))
}

#[cfg(not(unix))]
fn symlink(_target: &str, _link: &Path) -> Result<()> {
    bail!("Debian packages are built on Linux")
}

/// Directories 0755, binaries in `lib` 0755, other files 0644; symlinks keep
/// theirs. `dpkg-deb` only runs on Linux, so permissions are set on Unix only.
#[cfg(unix)]
fn set_modes(pkg: &Path, lib: &Path, apps: &[String]) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for entry in WalkDir::new(pkg) {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            continue;
        }
        let mode = if entry.file_type().is_dir()
            || entry.path().parent() == Some(lib)
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|n| apps.iter().any(|a| a == n))
        {
            0o755
        } else {
            0o644
        };
        fs::set_permissions(entry.path(), fs::Permissions::from_mode(mode))
            .with_context(|| format!("cannot set permissions on {}", entry.path().display()))?;
    }
    Ok(())
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "same signature as the Unix version"
)]
fn set_modes(_pkg: &Path, _lib: &Path, _apps: &[String]) -> Result<()> {
    Ok(())
}
