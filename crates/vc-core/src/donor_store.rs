//! Local donor-WAD library.
//!
//! A directory on disk where the user's own legitimately-owned official
//! WADs live, so they only need to be supplied once per title rather than
//! re-selected on every build. The store is created empty and is
//! populated entirely by the user importing their own files via
//! `DonorStore::import` — nothing is ever written here by this project
//! itself, and the directory is git-ignored (see `.gitignore`) so it can
//! never end up committed by accident.

use crate::error::VcError;
use std::fs;
use std::path::{Path, PathBuf};

pub struct DonorStore {
    root: PathBuf,
}

impl DonorStore {
    /// Open (creating if needed) a donor store rooted at the given
    /// directory. Typically a per-user app-data path chosen by the CLI/GUI
    /// shell, not anything bundled with the source tree.
    pub fn open_or_create(root: &Path) -> Result<Self, VcError> {
        fs::create_dir_all(root)?;
        Ok(DonorStore {
            root: root.to_path_buf(),
        })
    }

    /// Copy a user-selected WAD file into the store, named by its title
    /// id so it can be looked up later by any core that requires that
    /// exact title. Caller is expected to have already parsed the WAD
    /// (via `donor::parse_donor_wad`) to obtain `title_id` — this module
    /// only manages file placement, it doesn't parse WAD contents itself.
    pub fn import(&self, title_id: [u8; 8], source_path: &Path) -> Result<PathBuf, VcError> {
        let dest = self.path_for(title_id);
        fs::copy(source_path, &dest)?;
        Ok(dest)
    }

    /// Look up a previously-imported donor WAD by title id. Returns
    /// `None` if the user hasn't supplied that title yet — callers should
    /// surface this as a "please supply a donor WAD for X" prompt, not an
    /// error.
    pub fn lookup(&self, title_id: [u8; 8]) -> Option<PathBuf> {
        let path = self.path_for(title_id);
        path.exists().then_some(path)
    }

    /// List every title id currently present in the store.
    pub fn list(&self) -> Result<Vec<[u8; 8]>, VcError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(stem) = name.strip_suffix(".wad") {
                if let Some(id) = decode_hex8(stem) {
                    out.push(id);
                }
            }
        }
        Ok(out)
    }

    fn path_for(&self, title_id: [u8; 8]) -> PathBuf {
        self.root.join(format!("{}.wad", encode_hex8(title_id)))
    }
}

fn encode_hex8(id: [u8; 8]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex8(s: &str) -> Option<[u8; 8]> {
    if s.len() != 16 {
        return None;
    }
    let mut out = [0u8; 8];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
