//! Product and crate names, with the case variants the generator needs.

use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct NameError(pub String);

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NameError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductName {
    pub display: String,
    pub kebab: String,
    pub snake: String,
}

impl ProductName {
    /// # Errors
    /// When `s` breaks the naming rule.
    pub fn parse(s: &str) -> Result<Self, NameError> {
        check(s)?;
        Ok(Self {
            display: s.to_string(),
            kebab: join(s, '-'),
            snake: join(s, '_'),
        })
    }
}

/// The product's display name as a file name stem: spaces become `-`.
#[must_use]
pub fn file_stem(display: &str) -> String {
    display.replace(' ', "-")
}

const RULE: &str = "ASCII letters, digits, space, - or _; must start with a letter";

fn check(s: &str) -> Result<(), NameError> {
    let valid = s.starts_with(|c: char| c.is_ascii_alphabetic())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(NameError(format!("invalid name {s:?}: use {RULE}")))
    }
}

/// Lowercases `s` and joins its words with `sep`; space, `-` and `_` separate words.
fn join(s: &str, sep: char) -> String {
    let words: Vec<String> = s
        .split([' ', '-', '_'])
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    words.join(&sep.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateName {
    /// Kebab-case directory name.
    pub dir: String,
    pub snake: String,
}

impl CrateName {
    /// # Errors
    /// When `s` breaks the naming rule.
    pub fn parse(s: &str) -> Result<Self, NameError> {
        check(s)?;
        Ok(Self {
            dir: join(s, '-'),
            snake: join(s, '_'),
        })
    }

    /// The package name, prefixed with the product: `<product>-<dir>`.
    #[must_use]
    pub fn package(&self, p: &ProductName) -> String {
        format!("{}-{}", p.kebab, self.dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_cases() {
        let p = ProductName::parse("My App").unwrap();
        assert_eq!(
            (p.display.as_str(), p.kebab.as_str(), p.snake.as_str()),
            ("My App", "my-app", "my_app")
        );
        assert_eq!(ProductName::parse("my_app").unwrap().kebab, "my-app");
        assert_eq!(ProductName::parse("Pong2").unwrap().kebab, "pong2");
    }

    #[test]
    fn product_rejects() {
        for bad in ["", "2048", "-x", "a b!", "Müller"] {
            let e = ProductName::parse(bad).unwrap_err().to_string();
            assert!(
                e.contains("ASCII letters, digits, space, - or _; must start with a letter"),
                "{bad:?}: {e}"
            );
        }
    }

    #[test]
    fn crate_names_accept_reserved_words_and_prefix_the_package() {
        let p = ProductName::parse("My App").unwrap();
        for n in ["core", "std", "alloc", "test", "proc_macro"] {
            assert!(CrateName::parse(n).is_ok(), "{n}");
        }
        assert_eq!(CrateName::parse("core").unwrap().package(&p), "my-app-core");
        let c = CrateName::parse("proc_macro").unwrap();
        assert_eq!(
            (c.dir.as_str(), c.snake.as_str()),
            ("proc-macro", "proc_macro")
        );
    }
}
