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
    runtime: PathBuf,
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
        let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
        if runtime.as_ref().is_some_and(|path| !path.is_absolute()) {
            return Err(Error::UnsafePath(
                "XDG_RUNTIME_DIR must be an absolute path".into(),
            ));
        }
        Ok(Self::from_values_with_runtime(
            &home,
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
            runtime,
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
        Self::from_values_with_runtime(home, config, data, state, cache, None)
    }

    /// Resolve paths from explicit values including an optional private runtime root.
    #[must_use]
    pub fn from_values_with_runtime(
        home: &Path,
        config: Option<PathBuf>,
        data: Option<PathBuf>,
        state: Option<PathBuf>,
        cache: Option<PathBuf>,
        runtime: Option<PathBuf>,
    ) -> Self {
        let state = state.unwrap_or_else(|| home.join(".local/state"));
        let runtime = runtime.map_or_else(
            || state.join("mg-vault/runtime"),
            |path| path.join("mg-vault"),
        );
        Self {
            config: config
                .unwrap_or_else(|| home.join(".config"))
                .join("mg-vault"),
            data: data
                .unwrap_or_else(|| home.join(".local/share"))
                .join("mg-vault"),
            state: state.join("mg-vault"),
            cache: cache
                .unwrap_or_else(|| home.join(".cache"))
                .join("mg-vault"),
            runtime,
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
    pub fn runtime_dir(&self) -> PathBuf {
        self.runtime.clone()
    }

    #[must_use]
    pub fn service_socket(&self) -> PathBuf {
        self.runtime.join("indexd.sock")
    }

    #[must_use]
    pub fn registry_file(&self) -> PathBuf {
        self.config.join("vaults.json")
    }
}
