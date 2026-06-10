use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::domain::types::{PipelineJob, PipelineStageId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CivilDateTime {
    pub(crate) year: i32,
    pub(crate) month: u32,
    pub(crate) day: u32,
    pub(crate) hour: u32,
    pub(crate) minute: u32,
    pub(crate) second: u32,
}

pub(crate) fn civil_datetime(timestamp: u64) -> CivilDateTime {
    let days = (timestamp / 86_400) as i64;
    let seconds_of_day = (timestamp % 86_400) as u32;
    let (year, month, day) = civil_from_days(days);

    CivilDateTime {
        year,
        month,
        day,
        hour: seconds_of_day / 3_600,
        minute: (seconds_of_day % 3_600) / 60,
        second: seconds_of_day % 60,
    }
}

pub(crate) fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let shifted_days = days + 719_468;
    let era = match shifted_days >= 0 {
        true => shifted_days,
        false => shifted_days - 146_096,
    } / 146_097;
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = (year_of_era + era * 400) as i32;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_parameter = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_parameter + 2) / 5 + 1;
    let month = month_parameter
        + match month_parameter < 10 {
            true => 3,
            false => -9,
        };

    if month <= 2 {
        year += 1;
    }

    (year, month as u32, day as u32)
}

pub(crate) fn default_records_root() -> String {
    home_directory()
        .join("actavoces")
        .join("records")
        .display()
        .to_string()
}

pub(crate) fn default_model_storage_root() -> String {
    home_directory()
        .join("actavoces")
        .join("models")
        .display()
        .to_string()
}

pub(crate) fn ensure_configured_storage_directories(
    output_directory: &str,
    model_storage_directory: &str,
) -> rusqlite::Result<()> {
    ensure_directory(output_directory)?;
    ensure_directory(model_storage_directory)
}

pub(crate) fn ensure_directory(path: &str) -> rusqlite::Result<()> {
    fs::create_dir_all(path)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

pub(crate) fn remove_artifact_directory(path: &str) -> rusqlite::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(rusqlite::Error::ToSqlConversionFailure(Box::new(error))),
    }
}

pub(crate) fn home_directory() -> PathBuf {
    match env::var_os("HOME") {
        Some(home) => PathBuf::from(home),
        None => match env::var_os("USERPROFILE") {
            Some(home) => PathBuf::from(home),
            None => PathBuf::from("."),
        },
    }
}

pub(crate) fn option_number_to_string(value: Option<u8>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

pub(crate) fn parse_optional_number(value: &str) -> Option<u8> {
    if value.trim().is_empty() {
        return None;
    }

    value.parse().ok()
}

pub(crate) fn parse_bool(value: &str) -> bool {
    value == "true"
}

pub(crate) fn empty_string_to_none(value: String) -> Option<String> {
    match value.is_empty() {
        true => None,
        false => Some(value),
    }
}

pub(crate) fn json_string<T>(value: &T) -> rusqlite::Result<String>
where
    T: Serialize,
{
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

pub(crate) fn enum_value<T>(value: T) -> rusqlite::Result<String>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .and_then(|value| match value.as_str() {
            Some(value) => Ok(value.to_owned()),
            None => Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "enum did not serialize as a string",
            ))),
        })
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

pub(crate) fn enum_from_value<T>(value: &str) -> rusqlite::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(&format!("\"{value}\"")).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

pub(crate) fn row_to_pipeline_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<PipelineJob> {
    Ok(PipelineJob {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        stage: enum_from_value(&row.get::<_, String>(2)?)?,
        status: enum_from_value(&row.get::<_, String>(3)?)?,
        progress: row.get(4)?,
        message: row.get(5)?,
    })
}

pub(crate) fn pipeline_job_id(
    recording_id: &str,
    stage: PipelineStageId,
) -> Result<String, String> {
    let stage = enum_value(stage).map_err(|error| error.to_string())?;

    Ok(format!("{recording_id}-{stage}"))
}

pub(crate) fn read_json_file(path: PathBuf) -> Option<serde_json::Value> {
    let content = fs::read_to_string(path).ok()?;

    serde_json::from_str(&content).ok()
}

pub(crate) fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub(crate) fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    error.to_string()
}
