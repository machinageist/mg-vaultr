//! Filesystem-authority foundation for `mg-vault`.

mod atomic;
mod error;
mod registry;
mod vault;
mod xdg;

pub use error::{Error, Result};
pub use registry::{VaultRecord, VaultRegistry};
pub use vault::{Note, SourceFingerprint, TrashReceipt, Vault};
pub use xdg::XdgPaths;
