use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteValidationReport {
    pub integrity_check_passed: bool,
    pub missing_tables: Vec<String>,
    pub table_counts: Vec<(String, u64)>,
    pub errors: Vec<String>,
}

pub struct SqliteVerifier;

impl SqliteVerifier {
    pub const REQUIRED_TABLES: &'static [&'static str] = &[
        "session_meta",
        "raw_events",
        "canonical_actions",
        "screenshots",
        "video_segments",
        "annotations",
        "id_allocator",
    ];

    pub fn verify_database(path: &Path) -> Result<SqliteValidationReport, String> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|e| format!("Failed to open sqlite db at {}: {}", path.display(), e))?;

        let mut report = SqliteValidationReport {
            integrity_check_passed: false,
            missing_tables: Vec::new(),
            table_counts: Vec::new(),
            errors: Vec::new(),
        };

        // 1. Run PRAGMA integrity_check
        let integrity_res: Result<String, _> = conn.query_row("PRAGMA integrity_check;", [], |row| row.get(0));
        match integrity_res {
            Ok(ref val) if val == "ok" => {
                report.integrity_check_passed = true;
            }
            Ok(ref val) => {
                report.integrity_check_passed = false;
                report.errors.push(format!("PRAGMA integrity_check failed with: {}", val));
            }
            Err(e) => {
                report.integrity_check_passed = false;
                report.errors.push(format!("Failed to execute integrity_check: {}", e));
            }
        }

        // 2. Check table existence and row counts
        for &table in Self::REQUIRED_TABLES {
            let count_query = format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}';", table);
            let exists: u64 = conn.query_row(&count_query, [], |r| r.get(0)).unwrap_or(0);
            if exists == 0 {
                report.missing_tables.push(table.to_string());
            } else {
                let row_query = format!("SELECT COUNT(*) FROM {};", table);
                let rows: u64 = conn.query_row(&row_query, [], |r| r.get(0)).unwrap_or(0);
                report.table_counts.push((table.to_string(), rows));
            }
        }

        Ok(report)
    }
}
