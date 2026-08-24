use std::env;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Application-specific locations derived from the XDG Base Directory spec.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XdgPaths {
    config: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
}

impl XdgPaths {
    /// Resolve paths from the current process environment.
    ///
    /// # Errors
    ///
    /// Returns an error when `HOME` is unavailable.
    pub fn from_env() -> Result<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| Error::UnsafePath("HOME is not set".into()))?;
        Ok(Self::from_values(
            &home,
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
        ))
    }

    /// Resolve paths from explicit values, useful for isolated clients and tests.
    #[must_use]
    pub fn from_values(
        home: &Path,
        config: Option<PathBuf>,
        data: Option<PathBuf>,
        state: Option<PathBuf>,
        cache: Option<PathBuf>,
    ) -> Self {
        Self {
            config: config
                .unwrap_or_else(|| home.join(".config"))
                .join("mg-vault"),
            data: data
                .unwrap_or_else(|| home.join(".local/share"))
                .join("mg-vault"),
            state: state
                .unwrap_or_else(|| home.join(".local/state"))
                .join("mg-vault"),
            cache: cache
                .unwrap_or_else(|| home.join(".cache"))
                .join("mg-vault"),
        }
    }

    #[must_use]
    pub fn config_dir(&self) -> PathBuf {
        self.config.clone()
    }

    #[must_use]
    pub fn data_dir(&self) -> PathBuf {
        self.data.clone()
    }

    #[must_use]
    pub fn state_dir(&self) -> PathBuf {
        self.state.clone()
    }

    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.cache.clone()
    }

    #[must_use]
    pub fn registry_file(&self) -> PathBuf {
        self.config.join("vaults.json")
    }
}
