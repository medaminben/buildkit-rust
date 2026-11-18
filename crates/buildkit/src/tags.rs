/// A parsed `vX.Y.Z` or `vX.Y.Z-beta.N` tag.
struct Parsed<'a> {
    name: &'a str,
    key: (u64, u64, u64, bool, u64),
}

fn number(s: &str) -> Option<u64> {
    let ok =
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'));
    if ok { s.parse().ok() } else { None }
}

fn parse(tag: &str) -> Option<Parsed<'_>> {
    let rest = tag.strip_prefix('v')?;
    let (core, beta) = match rest.split_once("-beta.") {
        Some((core, n)) => (core, Some(number(n)?)),
        None => (rest, None),
    };
    let mut parts = core.split('.');
    let major = number(parts.next()?)?;
    let minor = number(parts.next()?)?;
    let patch = number(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(Parsed {
        name: tag,
        key: (major, minor, patch, beta.is_none(), beta.unwrap_or(0)),
    })
}

/// The tag a ref entry such as `tag: refs/tags/v1.0.0` names, if it is a version tag.
fn tag_of(entry: &str) -> Option<Parsed<'_>> {
    const P: &str = "refs/tags/";
    entry
        .match_indices(P)
        .find_map(|(i, _)| parse(&entry[i + P.len()..]))
}

/// Picks the nearest version tag from the output of
/// `git log --simplify-by-decoration --decorate=full --format=%D HEAD`.
///
/// The first line (nearest commit) holding a version tag decides. On that
/// line a stable tag beats any beta, then the highest version wins.
#[must_use]
pub fn select_tag(decorations: &str) -> Option<String> {
    for line in decorations.lines() {
        let found: Vec<Parsed> = line.split(", ").filter_map(tag_of).collect();
        let stable_only = found.iter().any(|p| p.key.3);
        let best = found
            .iter()
            .filter(|p| !stable_only || p.key.3)
            .max_by_key(|p| p.key);
        if let Some(p) = best {
            return Some(p.name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::select_tag;

    fn sel(s: &str) -> Option<String> {
        select_tag(s)
    }

    #[test]
    fn head_and_tag() {
        assert_eq!(
            sel("HEAD -> refs/heads/main, tag: refs/tags/v1.0.0").as_deref(),
            Some("v1.0.0")
        );
    }

    #[test]
    fn empty_and_unrelated() {
        assert_eq!(sel(""), None);
        assert_eq!(
            sel("HEAD -> refs/heads/main\ntag: refs/tags/release-7"),
            None
        );
        assert_eq!(sel("tag: refs/tags/v9.9.9-rc.1"), None);
    }

    #[test]
    fn nearest_line_wins() {
        let d = "tag: refs/tags/v1.1.0\n\ntag: refs/tags/v1.0.0";
        assert_eq!(sel(d).as_deref(), Some("v1.1.0"));
        let d = "HEAD -> refs/heads/main\n\ntag: refs/tags/v1.0.0";
        assert_eq!(sel(d).as_deref(), Some("v1.0.0"));
    }

    #[test]
    fn stable_beats_beta_in_either_order() {
        assert_eq!(
            sel("tag: refs/tags/v1.6.0, tag: refs/tags/v1.6.0-beta.3").as_deref(),
            Some("v1.6.0")
        );
        assert_eq!(
            sel("tag: refs/tags/v1.6.0-beta.3, tag: refs/tags/v1.6.0").as_deref(),
            Some("v1.6.0")
        );
        // Stable wins even over a higher beta, as in the CMake reference.
        assert_eq!(
            sel("tag: refs/tags/v1.6.0-beta.1, tag: refs/tags/v1.5.0").as_deref(),
            Some("v1.5.0")
        );
    }

    #[test]
    fn highest_numeric_wins() {
        assert_eq!(
            sel("tag: refs/tags/v1.6.0-beta.2, tag: refs/tags/v1.6.0-beta.10").as_deref(),
            Some("v1.6.0-beta.10")
        );
        assert_eq!(
            sel("tag: refs/tags/v1.9.0, tag: refs/tags/v1.10.0").as_deref(),
            Some("v1.10.0")
        );
    }

    #[test]
    fn leading_zeros_and_junk_rejected() {
        for t in [
            "v01.2.3",
            "v1.2.3-beta.01",
            "v1.2",
            "v1.2.3.4",
            "v1.2.3-beta",
            "1.2.3",
            "v1.2.3-rc.1",
            "v1.2.3-beta.1x",
        ] {
            assert_eq!(sel(&format!("tag: refs/tags/{t}")), None, "{t}");
        }
        assert_eq!(sel("tag: refs/tags/v0.0.0").as_deref(), Some("v0.0.0"));
    }
}
