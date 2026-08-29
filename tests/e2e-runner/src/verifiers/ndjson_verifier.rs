use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdjsonValidationReport {
    pub total_lines: usize,
    pub valid_lines: usize,
    pub corrupted_lines: usize,
    pub min_global_event_id: Option<u64>,
    pub max_global_event_id: Option<u64>,
    pub is_strictly_monotonic: bool,
    pub sensitive_leaks_detected: Vec<String>,
    pub errors: Vec<String>,
}

pub struct NdjsonVerifier;

impl NdjsonVerifier {
    pub fn verify_raw_ndjson(path: &Path, forbidden_plaintext_tokens: &[&str]) -> Result<NdjsonValidationReport, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
        let reader = BufReader::new(file);

        let mut report = NdjsonValidationReport {
            total_lines: 0,
            valid_lines: 0,
            corrupted_lines: 0,
            min_global_event_id: None,
            max_global_event_id: None,
            is_strictly_monotonic: true,
            sensitive_leaks_detected: Vec::new(),
            errors: Vec::new(),
        };

        let mut last_event_id: Option<u64> = None;
        let mut last_monotonic_ns: Option<u64> = None;

        for (idx, line_res) in reader.lines().enumerate() {
            report.total_lines += 1;
            let line_num = idx + 1;
            let line = match line_res {
                Ok(l) => l,
                Err(e) => {
                    report.corrupted_lines += 1;
                    report.errors.push(format!("Line {}: Read I/O error: {}", line_num, e));
                    continue;
                }
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check for forbidden plaintext strings directly in raw line
            for token in forbidden_plaintext_tokens {
                if trimmed.contains(token) {
                    report.sensitive_leaks_detected.push(format!(
                        "Line {}: Plaintext leak detected for token '{}'",
                        line_num, token
                    ));
                }
            }

            // Parse JSON object
            let val: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    report.corrupted_lines += 1;
                    report.errors.push(format!("Line {}: Invalid JSON syntax: {}", line_num, e));
                    continue;
                }
            };

            // Check global_event_id
            if let Some(event_id) = val.get("global_event_id").and_then(|v| v.as_u64()) {
                if report.min_global_event_id.is_none() {
                    report.min_global_event_id = Some(event_id);
                }
                report.max_global_event_id = Some(event_id);

                if let Some(prev) = last_event_id {
                    if event_id <= prev {
                        report.is_strictly_monotonic = false;
                        report.errors.push(format!(
                            "Line {}: Non-monotonic global_event_id (current: {}, previous: {})",
                            line_num, event_id, prev
                        ));
                    }
                }
                last_event_id = Some(event_id);
            }

            // Check timestamp.monotonic_ns
            if let Some(mono_ns) = val
                .get("timestamp")
                .and_then(|t| t.get("monotonic_ns"))
                .and_then(|m| m.as_u64())
            {
                if let Some(prev_mono) = last_monotonic_ns {
                    if mono_ns < prev_mono {
                        report.errors.push(format!(
                            "Line {}: Decreasing monotonic_ns (current: {}, previous: {})",
                            line_num, mono_ns, prev_mono
                        ));
                    }
                }
                last_monotonic_ns = Some(mono_ns);
            }

            report.valid_lines += 1;
        }

        Ok(report)
    }

    pub fn verify_cross_stream_consistency(
        raw_report: &NdjsonValidationReport,
        normalized_report: &NdjsonValidationReport,
    ) -> Result<(), String> {
        if !raw_report.sensitive_leaks_detected.is_empty() {
            return Err(format!(
                "Raw stream failed privacy check with {} leaks",
                raw_report.sensitive_leaks_detected.len()
            ));
        }
        if !normalized_report.sensitive_leaks_detected.is_empty() {
            return Err(format!(
                "Normalized stream failed privacy check with {} leaks",
                normalized_report.sensitive_leaks_detected.len()
            ));
        }
        if raw_report.corrupted_lines > 0 {
            return Err(format!(
                "Raw stream contains {} corrupted lines",
                raw_report.corrupted_lines
            ));
        }
        if normalized_report.corrupted_lines > 0 {
            return Err(format!(
                "Normalized stream contains {} corrupted lines",
                normalized_report.corrupted_lines
            ));
        }
        Ok(())
    }
}
