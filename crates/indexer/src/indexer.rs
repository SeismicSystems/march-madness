//! Core indexer logic: reading/writing the index file with file locking.

use eyre::{Result, WrapErr};
use fs2::FileExt;
use march_madness_common::{EntryIndex, EntryRecord, UpdateInfo};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Load the index from disk (returns empty index if file doesn't exist).
pub fn load_index(path: &Path) -> Result<EntryIndex> {
    if !path.exists() {
        return Ok(EntryIndex::new());
    }
    let file = fs::File::open(path).wrap_err("failed to open index file")?;
    file.lock_shared()
        .wrap_err("failed to acquire shared lock")?;
    let mut content = String::new();
    let mut reader = std::io::BufReader::new(&file);
    reader
        .read_to_string(&mut content)
        .wrap_err("failed to read index file")?;
    file.unlock().ok();

    if content.trim().is_empty() {
        return Ok(EntryIndex::new());
    }
    serde_json::from_str(&content).wrap_err("failed to parse index JSON")
}

/// Write the index to disk with an exclusive file lock.
pub fn save_index(path: &Path, index: &EntryIndex) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err("failed to create index directory")?;
    }

    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .wrap_err("failed to open index file for writing")?;

    file.lock_exclusive()
        .wrap_err("failed to acquire exclusive lock")?;

    let json = serde_json::to_string_pretty(index).wrap_err("failed to serialize index")?;
    let mut writer = std::io::BufWriter::new(&file);
    writer
        .write_all(json.as_bytes())
        .wrap_err("failed to write index file")?;
    writer.flush().wrap_err("failed to flush index file")?;

    file.unlock().ok();
    Ok(())
}

/// Insert or update an entry for a BracketSubmitted event.
pub fn upsert_bracket_submitted(index: &mut EntryIndex, address: &str, block: u64, timestamp: u64) {
    let key = address.to_lowercase();
    let entry = index.entry(key).or_insert_with(|| EntryRecord {
        name: None,
        updated: UpdateInfo { block: 0, ts: 0 },
        bracket: None,
    });
    entry.updated = UpdateInfo {
        block,
        ts: timestamp,
    };
}

/// Update the tag/name for an address.
pub fn update_tag(index: &mut EntryIndex, address: &str, tag: String) {
    let key = address.to_lowercase();
    let entry = index.entry(key).or_insert_with(|| EntryRecord {
        name: None,
        updated: UpdateInfo { block: 0, ts: 0 },
        bracket: None,
    });
    entry.name = Some(tag);
}

/// Set the bracket hex for an address (after reveal).
pub fn set_bracket(index: &mut EntryIndex, address: &str, bracket_hex: String) {
    let key = address.to_lowercase();
    if let Some(entry) = index.get_mut(&key) {
        entry.bracket = Some(bracket_hex);
    }
}
