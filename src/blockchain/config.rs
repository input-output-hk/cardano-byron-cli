//! local blockchain configuration related functions and tools
//!

use std::path::{Path, PathBuf};

/// this is the name of the directory where the blockchains'
/// blocks, epochs and tags will lie.
pub const BLOCKCHAINS_DIRECTORY: &'static str = "blockchains";

pub fn blockchains_directory<P: AsRef<Path>>(root_dir: P) -> PathBuf {
    root_dir.as_ref().join(BLOCKCHAINS_DIRECTORY)
}

/// handy function to define where to find the blockchains related
/// functions in a given _cardano-cli_ directory.
///
pub fn directory<P: AsRef<Path>>(root_dir: P, name: &str) -> PathBuf {
    root_dir.as_ref().join(BLOCKCHAINS_DIRECTORY).join(name)
}
