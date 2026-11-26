use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use textfsm_core::{CliTable, CliTableError};

use super::to_records::to_records;
use crate::domain::textfsm::ntc_platform::ntc_platform;
use crate::types::textfsm_record::TextFsmRecord;
use crate::{DeviceType, Error, Result};

const INDEX_FILE: &str = "index";

// Compiling the full ntc index takes ~0.3 s (release), so keep it per directory like Python's clitable does.
static CLI_TABLES: LazyLock<Mutex<HashMap<PathBuf, Arc<CliTable>>>> = LazyLock::new(Mutex::default);

/// Picks the template from `templates_dir/index` by platform and command (abbreviations
/// like `sh ver` included) and parses `output`. `None` when no template matches.
pub fn parse_ntc(
    templates_dir: &Path,
    device_type: DeviceType,
    command: &str,
    output: &str,
) -> Result<Option<Vec<TextFsmRecord>>> {
    let cli_table = cached_cli_table(templates_dir)?;
    let attributes = HashMap::from([
        ("Command".to_string(), command.trim().to_string()),
        ("Platform".to_string(), ntc_platform(device_type).to_string()),
    ]);
    match cli_table.parse_cmd(output, &attributes) {
        Ok(table) => {
            let header = table.header().to_vec();
            to_records(&header, table.into_values()).map(Some)
        }
        Err(CliTableError::NoMatch(_)) => Ok(None),
        Err(error) => Err(Error::TextFsm(error.to_string())),
    }
}

/// A changed index or template is only picked up by a new process (Python's clitable caches the index only).
fn cached_cli_table(templates_dir: &Path) -> Result<Arc<CliTable>> {
    let mut cli_tables = CLI_TABLES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(cli_table) = cli_tables.get(templates_dir) {
        return Ok(Arc::clone(cli_table));
    }
    let cli_table = CliTable::new(templates_dir.join(INDEX_FILE), templates_dir)
        .map_err(|error| Error::TextFsm(error.to_string()))?;
    let cli_table = Arc::new(cli_table);
    cli_tables.insert(templates_dir.to_path_buf(), Arc::clone(&cli_table));
    Ok(cli_table)
}
