//! Filesystem-authority foundation for `mg-vault`.

mod atomic;
mod error;
mod frontmatter;
mod registry;
mod vault;
mod xdg;

pub use error::{Error, Result};
pub use frontmatter::{FrontmatterEnvelope, FrontmatterError, LineEndings, scan_frontmatter};
pub use registry::{VaultRecord, VaultRegistry};
pub use vault::{Note, SourceFingerprint, TrashReceipt, Vault};
pub use xdg::XdgPaths;
