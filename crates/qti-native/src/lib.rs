//! Native host services around the portable conversion engines.

mod assets;
mod error;
pub mod html_to_image;
mod input;
mod metadata;
mod persistence;
mod ple_output;

pub use assets::DirectoryAssets;
pub use error::NativeError;
pub use input::{check_package_path, read_package_entries, read_source};
pub use metadata::resolve_write_context;
pub use persistence::persist_artifact;
