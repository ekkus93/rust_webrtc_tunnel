//! Path expansion and file-security validation helpers used while loading and
//! validating an [`AppConfig`](super::AppConfig): `~/` home expansion, required/
//! optional file existence checks, and world-writable permission rejection.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ConfigError;
pub fn expand_home(path: &Path) -> Result<PathBuf, ConfigError> {
    let path_string = path.to_string_lossy();
    if !path_string.starts_with("~/") {
        return Ok(path.to_path_buf());
    }

    let home = env::var_os("HOME").ok_or_else(|| {
        ConfigError::InvalidConfig("HOME environment variable is not set".to_owned())
    })?;

    let relative = path_string.trim_start_matches("~/");
    Ok(PathBuf::from(home).join(relative))
}

pub(crate) fn expand_optional_path(path: &Path) -> Result<PathBuf, ConfigError> {
    if path.as_os_str().is_empty() {
        return Ok(PathBuf::new());
    }

    expand_home(path)
}

pub(crate) fn validate_required_file(
    path: &Path,
    field_name: &'static str,
) -> Result<(), ConfigError> {
    validate_optional_file(path, field_name, true)
}

pub(crate) fn validate_optional_file(
    path: &Path,
    field_name: &'static str,
    required: bool,
) -> Result<(), ConfigError> {
    if path.as_os_str().is_empty() {
        if required {
            return Err(ConfigError::InvalidConfig(format!("{field_name} must be set")));
        }
        return Ok(());
    }
    if !path.is_file() {
        return Err(ConfigError::InvalidConfig(format!(
            "{field_name} file '{}' does not exist",
            path.display()
        )));
    }
    Ok(())
}
#[cfg(unix)]
pub(crate) fn validate_non_world_writable(
    path: &Path,
    field_name: &'static str,
) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;

    if path.as_os_str().is_empty() {
        return Ok(());
    }

    let mut candidate = path;
    while !candidate.exists() {
        candidate = candidate.parent().ok_or_else(|| {
            ConfigError::InvalidConfig(format!(
                "{field_name} must be inside an existing directory for path security checks"
            ))
        })?;
    }

    let metadata =
        fs::metadata(candidate).map_err(|error| ConfigError::io_path(candidate, error))?;
    if metadata.permissions().mode() & 0o002 != 0 {
        return Err(ConfigError::InvalidConfig(format!(
            "{field_name} path '{}' must not be world-writable",
            candidate.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn validate_non_world_writable(
    _path: &Path,
    _field_name: &'static str,
) -> Result<(), ConfigError> {
    Ok(())
}
