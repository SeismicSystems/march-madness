//! Atomic file writer for tournament-status.json.

use std::path::Path;

use eyre::{Context, Result};
use tracing::debug;

/// Atomically write tournament status JSON to a file.
///
/// Writes to a temp file first, then renames. This prevents partial reads.
pub fn write_tournament_status(
    path: &Path,
    status: &seismic_march_madness::TournamentStatus,
) -> Result<()> {
    let json =
        serde_json::to_string_pretty(status).wrap_err("failed to serialize tournament status")?;

    let tmp_path = path.with_extension("json.tmp");

    std::fs::write(&tmp_path, &json)
        .wrap_err_with(|| format!("failed to write {}", tmp_path.display()))?;

    std::fs::rename(&tmp_path, path).wrap_err_with(|| {
        format!(
            "failed to rename {} → {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    debug!("wrote tournament status to {}", path.display());
    Ok(())
}
