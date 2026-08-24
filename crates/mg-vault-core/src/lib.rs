//! Filesystem-authority foundation for `mg-vault`.

mod atomic;
mod error;
mod frontmatter;
mod frontmatter_scalar;
mod registry;
mod vault;
mod xdg;

pub use error::{Error, Result};
pub use frontmatter::{FrontmatterEnvelope, FrontmatterError, LineEndings, scan_frontmatter};
pub use frontmatter_scalar::{FrontmatterScalarError, locate_frontmatter_scalar};
pub use registry::{VaultRecord, VaultRegistry};
pub use vault::{Note, SourceFingerprint, TrashReceipt, Vault};
pub use xdg::XdgPaths;
