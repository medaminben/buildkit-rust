//! The product workspace: its root manifest and the buildkit metadata in it.

use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item};

pub struct Workspace {
    pub root: PathBuf,
    pub doc: DocumentMut,
}

pub struct PackageMeta {
    pub display_name: String,
    pub main_app: String,
    pub apps: Vec<String>,
    pub iss: PathBuf,
    pub desktop: PathBuf,
    pub icon: PathBuf,
    pub extra_files: Vec<String>,
    pub system_deps_apt: Vec<String>,
}

impl Workspace {
    /// Nearest ancestor of `start` whose `Cargo.toml` has a `[workspace]`.
    ///
    /// # Errors
    /// When there is none, or a manifest cannot be read or parsed.
    pub fn find(start: &Path) -> Result<Workspace> {
        for dir in start.ancestors() {
            let manifest = dir.join("Cargo.toml");
            if !manifest.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&manifest)
                .with_context(|| format!("cannot read {}", manifest.display()))?;
            let doc: DocumentMut = text
                .parse()
                .with_context(|| format!("cannot parse {}", manifest.display()))?;
            if doc.contains_key("workspace") {
                return Ok(Workspace {
                    root: dir.to_path_buf(),
                    doc,
                });
            }
        }
        bail!(
            "no Cargo workspace found from {}; run cargo bk inside a product",
            start.display()
        )
    }

    /// `[workspace.metadata.buildkit].version`.
    #[must_use]
    pub fn bk_version(&self) -> Option<String> {
        self.metadata()?
            .get("buildkit")?
            .get("version")?
            .as_str()
            .map(str::to_string)
    }

    fn metadata(&self) -> Option<&Item> {
        self.doc.get("workspace")?.get("metadata")
    }

    /// `[workspace.metadata.bk.package]`.
    ///
    /// # Errors
    /// When a required key is missing.
    pub fn package_meta(&self) -> Result<PackageMeta> {
        let table = self
            .metadata()
            .and_then(|m| m.get("bk"))
            .and_then(|b| b.get("package"))
            .ok_or_else(|| anyhow!("[workspace.metadata.bk.package] is missing"))?;
        let string = |key: &str| -> Result<String> {
            table
                .get(key)
                .and_then(Item::as_str)
                .map(str::to_string)
                .ok_or_else(|| anyhow!("[workspace.metadata.bk.package] {key} is missing"))
        };
        let strings = |a: &toml_edit::Array| -> Vec<String> {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        };
        let list = |key: &str, required: bool| -> Result<Vec<String>> {
            match table.get(key).and_then(Item::as_array) {
                Some(a) => Ok(strings(a)),
                None if required => bail!("[workspace.metadata.bk.package] {key} is missing"),
                None => Ok(Vec::new()),
            }
        };
        Ok(PackageMeta {
            display_name: string("display-name")?,
            main_app: string("main-app")?,
            apps: list("apps", true)?,
            iss: string("iss")?.into(),
            desktop: string("desktop")?.into(),
            icon: string("icon")?.into(),
            extra_files: list("extra-files", false)?,
            system_deps_apt: table
                .get("system-deps")
                .and_then(|d| d.get("apt"))
                .and_then(Item::as_array)
                .map_or_default(strings),
        })
    }

    /// `[workspace.metadata.buildkit] repo` as `owner/repo`; empty when unset.
    #[must_use]
    pub fn repo(&self) -> String {
        self.metadata()
            .and_then(|m| m.get("buildkit"))
            .and_then(|b| b.get("repo"))
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_string()
    }

    /// Writes the manifest back, keeping its formatting.
    ///
    /// # Errors
    /// When the file cannot be written.
    pub fn save(&self) -> Result<()> {
        let path = self.root.join("Cargo.toml");
        std::fs::write(&path, self.doc.to_string())
            .with_context(|| format!("cannot write {}", path.display()))
    }

    /// The root package's version, if the root manifest has a `[package]`.
    #[must_use]
    pub fn root_package_version(&self) -> Option<String> {
        self.doc
            .get("package")?
            .get("version")?
            .as_str()
            .map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn ws(dir: &Path, body: &str) {
        fs::write(dir.join("Cargo.toml"), body).unwrap();
    }

    #[test]
    fn find_from_nested_dir() {
        let t = tempfile::tempdir().unwrap();
        ws(t.path(), "[workspace]\nmembers = []\n");
        let nested = t.path().join("a/b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(t.path().join("a/Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        let w = Workspace::find(&nested).unwrap();
        assert_eq!(w.root, t.path());
    }

    #[test]
    fn find_without_workspace_errors() {
        let t = tempfile::tempdir().unwrap();
        let e = Workspace::find(t.path()).err().unwrap().to_string();
        assert_eq!(
            e,
            format!(
                "no Cargo workspace found from {}; run cargo bk inside a product",
                t.path().display()
            )
        );
    }

    #[test]
    fn bk_version_and_missing_package_key() {
        let t = tempfile::tempdir().unwrap();
        ws(
            t.path(),
            "[workspace]\n[workspace.metadata.buildkit]\nversion = \"1.2.3\"\n[workspace.metadata.bk.package]\ndisplay-name = \"X\"\n",
        );
        let w = Workspace::find(t.path()).unwrap();
        assert_eq!(w.bk_version().as_deref(), Some("1.2.3"));
        let e = w.package_meta().err().unwrap().to_string();
        assert!(
            e.contains("[workspace.metadata.bk.package] main-app is missing"),
            "{e}"
        );
    }

    #[test]
    fn package_meta_reads_all_keys_and_save_keeps_formatting() {
        let t = tempfile::tempdir().unwrap();
        let body = "[workspace]\n# keep me\n[workspace.metadata.bk.package]\ndisplay-name = \"X\"\nmain-app = \"x\"\napps = [\"x\"]\niss = \"a.iss\"\ndesktop = \"a.desktop\"\nicon = \"a.png\"\nextra-files = [\"LICENSE\"]\nsystem-deps.apt = [\"libfoo\"]\n";
        ws(t.path(), body);
        let w = Workspace::find(t.path()).unwrap();
        let m = w.package_meta().unwrap();
        assert_eq!(m.display_name, "X");
        assert_eq!(m.apps, ["x"]);
        assert_eq!(m.icon, PathBuf::from("a.png"));
        assert_eq!(m.system_deps_apt, ["libfoo"]);
        let mut w = w;
        w.doc["workspace"]["metadata"]["bk"]["package"]["icon"] = toml_edit::value("b.png");
        w.save().unwrap();
        let saved = fs::read_to_string(t.path().join("Cargo.toml")).unwrap();
        assert_eq!(saved, body.replace("a.png", "b.png"));
        assert!(saved.contains("# keep me"));
        let pos = |k: &str| saved.find(k).unwrap();
        assert!(pos("display-name") < pos("main-app") && pos("main-app") < pos("icon"));
    }
}
