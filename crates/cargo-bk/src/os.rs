//! The operating systems the generated products target.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Linux,
    Windows,
}

impl Os {
    /// The OS this binary runs on; anything that is not Windows counts as Linux.
    #[must_use]
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}
