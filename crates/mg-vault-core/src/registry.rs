use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::replace_atomic;
use crate::{Error, Result};

/// A named, canonical vault root persisted in the user registry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VaultRecord {
    pub name: String,
    pub path: PathBuf,
}

/// XDG configuration registry for independent vault roots.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VaultRegistry {
    #[serde(skip)]
    path: PathBuf,
    #[serde(default)]
    vaults: BTreeMap<String, VaultRecord>,
    selected: Option<String>,
}

impl VaultRegistry {
    /// Load a registry, or return an empty registry when the file is absent.
    ///
    /// # Errors
    ///
    /// Returns an error for unreadable or malformed registry data.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                path: path.to_path_buf(),
                ..Self::default()
            });
        }
        let bytes = fs::read(path).map_err(|error| Error::io(path, error))?;
        let mut registry: Self = serde_json::from_slice(&bytes)?;
        registry.path = path.to_path_buf();
        Ok(registry)
    }

    /// Add a canonical directory under a unique registry name.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid names, inaccessible/non-directory paths,
    /// duplicate names, or a registry persistence failure.
    pub fn register(&mut self, name: &str, path: &Path) -> Result<VaultRecord> {
        validate_name(name)?;
        let canonical = fs::canonicalize(path).map_err(|error| Error::io(path, error))?;
        let metadata = fs::metadata(&canonical).map_err(|error| Error::io(&canonical, error))?;
        if !metadata.is_dir() {
            return Err(Error::UnsafePath(format!(
                "{} is not a directory",
                path.display()
            )));
        }
        if self.vaults.contains_key(name) {
            return Err(Error::Collision(self.path.clone()));
        }
        let record = VaultRecord {
            name: name.to_owned(),
            path: canonical,
        };
        self.vaults.insert(name.to_owned(), record.clone());
        if self.selected.is_none() {
            self.selected = Some(name.to_owned());
        }
        self.save()?;
        Ok(record)
    }

    /// Select an existing registry entry and persist the choice.
    ///
    /// # Errors
    ///
    /// Returns an error when the name is unknown or persistence fails.
    pub fn select(&mut self, name: &str) -> Result<()> {
        if !self.vaults.contains_key(name) {
            return Err(Error::UnknownVault(name.to_owned()));
        }
        self.selected = Some(name.to_owned());
        self.save()
    }

    #[must_use]
    pub fn list(&self) -> Vec<VaultRecord> {
        self.vaults.values().cloned().collect()
    }

    #[must_use]
    pub fn selected_name(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Resolve an explicit name or the current selection to its vault root.
    ///
    /// # Errors
    ///
    /// Returns an error when no vault is selected or the name is unknown.
    pub fn resolve(&self, requested: Option<&str>) -> Result<PathBuf> {
        let name = requested
            .or(self.selected.as_deref())
            .ok_or(Error::NoVaultSelected)?;
        self.vaults
            .get(name)
            .map(|record| record.path.clone())
            .ok_or_else(|| Error::UnknownVault(name.to_owned()))
    }

    fn save(&self) -> Result<()> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| Error::UnsafePath(format!("{} has no parent", self.path.display())))?;
        fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        let bytes = serde_json::to_vec_pretty(self)?;
        if self.path.exists() {
            replace_atomic(&self.path, &bytes)
        } else {
            crate::atomic::create_atomic(&self.path, &bytes)
        }
    }
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(Error::InvalidVaultName(name.to_owned()));
    }
    Ok(())
}
