use std::convert::AsRef;
use std::fs::{self, File, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::filter::{Action, ActionFuncPair, Filter, FilterItem, parse_rule_text_with_diagnostics};
use crate::{wm_error, wm_trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SheetFormat {
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "rule")]
    Plain,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "bin")]
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetDecodeError {
    pub errors: Vec<String>,
}

impl SheetDecodeError {
    fn new<S: Into<String>>(message: S) -> Self {
        Self {
            errors: vec![message.into()],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SheetSummary {
    pub rule_count: usize,
    pub pin_rule_count: usize,
    pub filter_rule_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SheetFileReport {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<SheetFormat>,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin_rule_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_rule_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

impl SheetFileReport {
    fn valid(path: &Path, format: SheetFormat, summary: SheetSummary) -> Self {
        Self {
            path: display_path(path),
            format: Some(format),
            valid: true,
            rule_count: Some(summary.rule_count),
            pin_rule_count: Some(summary.pin_rule_count),
            filter_rule_count: Some(summary.filter_rule_count),
            errors: Vec::new(),
        }
    }

    fn invalid(path: &Path, format: Option<SheetFormat>, errors: Vec<String>) -> Self {
        Self {
            path: display_path(path),
            format,
            valid: false,
            rule_count: None,
            pin_rule_count: None,
            filter_rule_count: None,
            errors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SheetVerificationReport {
    pub target: String,
    pub recursive: bool,
    pub sheets_found: usize,
    pub valid_sheets: usize,
    pub invalid_sheets: usize,
    pub all_valid: bool,
    pub files: Vec<SheetFileReport>,
    pub errors: Vec<String>,
}

impl SheetVerificationReport {
    fn new(target: &Path) -> Self {
        Self {
            target: display_path(target),
            recursive: true,
            sheets_found: 0,
            valid_sheets: 0,
            invalid_sheets: 0,
            all_valid: false,
            files: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn finalize(mut self) -> Self {
        self.sheets_found = self.files.len();
        self.valid_sheets = self.files.iter().filter(|file| file.valid).count();
        self.invalid_sheets = self.files.iter().filter(|file| !file.valid).count();
        self.all_valid =
            self.errors.is_empty() && self.invalid_sheets == 0 && self.sheets_found > 0;
        self
    }

    pub fn has_failures(&self) -> bool {
        !self.all_valid
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn unsupported_format_message(path: &Path) -> String {
    format!(
        "unsupported sheet format for {}; expected .rule, .json, or .bin",
        display_path(path)
    )
}

pub fn detect_sheet_format<P: AsRef<Path>>(path: P) -> Option<SheetFormat> {
    match path
        .as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        Some(ext) if ext == "rule" => Some(SheetFormat::Plain),
        Some(ext) if ext == "json" => Some(SheetFormat::Json),
        Some(ext) if ext == "bin" => Some(SheetFormat::Binary),
        _ => None,
    }
}

fn decode_plain_sheet(data: &str) -> Result<Vec<FilterItem>, SheetDecodeError> {
    parse_rule_text_with_diagnostics(data).map_err(|errors| SheetDecodeError { errors })
}

fn decode_text_sheet(
    data: &[u8],
    format: SheetFormat,
) -> Result<Vec<FilterItem>, SheetDecodeError> {
    let content = std::str::from_utf8(data)
        .map_err(|err| SheetDecodeError::new(format!("sheet data is not valid UTF-8: {}", err)))?;

    match format {
        SheetFormat::Plain => decode_plain_sheet(content),
        SheetFormat::Json => serde_json::from_str::<Vec<FilterItem>>(content)
            .map_err(|err| SheetDecodeError::new(format!("failed to parse json sheet: {}", err))),
        SheetFormat::Binary | SheetFormat::Invalid => Err(SheetDecodeError::new(
            "text decoding is not supported for this sheet format",
        )),
    }
}

fn decode_sheet_bytes(
    data: &[u8],
    format: SheetFormat,
) -> Result<Vec<FilterItem>, SheetDecodeError> {
    match format {
        SheetFormat::Plain | SheetFormat::Json => decode_text_sheet(data, format),
        SheetFormat::Binary => bincode::deserialize::<Vec<FilterItem>>(data)
            .map_err(|err| SheetDecodeError::new(format!("failed to parse binary sheet: {}", err))),
        SheetFormat::Invalid => Err(SheetDecodeError::new("invalid sheet format")),
    }
}

pub fn decode_sheet<P: AsRef<Path>>(path: P) -> Result<Vec<FilterItem>, SheetDecodeError> {
    let path = path.as_ref();
    let format = detect_sheet_format(path)
        .ok_or_else(|| SheetDecodeError::new(unsupported_format_message(path)))?;

    let bytes = fs::read(path).map_err(|err| {
        SheetDecodeError::new(format!("failed to read {}: {}", display_path(path), err))
    })?;
    decode_sheet_bytes(&bytes, format)
}

pub fn summarize_items(items: &[FilterItem]) -> SheetSummary {
    SheetSummary {
        rule_count: items.len(),
        pin_rule_count: items
            .iter()
            .filter(|item| item.action == Action::Pin)
            .count(),
        filter_rule_count: items
            .iter()
            .filter(|item| item.action == Action::FilterOut)
            .count(),
    }
}

fn collect_sheet_paths(root: &Path, collected: &mut Vec<PathBuf>, errors: &mut Vec<String>) {
    let mut entries = match fs::read_dir(root) {
        Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(err) => {
            errors.push(format!(
                "failed to read directory {}: {}",
                display_path(root),
                err
            ));
            return;
        }
    };
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) => {
                errors.push(format!(
                    "failed to inspect {}: {}",
                    display_path(&path),
                    err
                ));
                continue;
            }
        };

        let file_type = metadata.file_type();
        if file_type.is_symlink() && path.is_dir() {
            continue;
        }

        if path.is_dir() {
            collect_sheet_paths(&path, collected, errors);
            continue;
        }

        if detect_sheet_format(&path).is_some() {
            collected.push(path);
        }
    }
}

pub fn verify_target<P: AsRef<Path>>(path: P) -> SheetVerificationReport {
    let path = path.as_ref();
    let mut report = SheetVerificationReport::new(path);

    if !path.exists() {
        report
            .errors
            .push(format!("target does not exist: {}", display_path(path)));
        return report.finalize();
    }

    if path.is_file() {
        let Some(format) = detect_sheet_format(path) else {
            report.errors.push(unsupported_format_message(path));
            return report.finalize();
        };

        match decode_sheet(path) {
            Ok(items) => {
                report.files.push(SheetFileReport::valid(
                    path,
                    format,
                    summarize_items(&items),
                ));
            }
            Err(err) => {
                report
                    .files
                    .push(SheetFileReport::invalid(path, Some(format), err.errors));
            }
        }

        return report.finalize();
    }

    if !path.is_dir() {
        report.errors.push(format!(
            "target is not a file or directory: {}",
            display_path(path)
        ));
        return report.finalize();
    }

    let mut candidates = Vec::new();
    collect_sheet_paths(path, &mut candidates, &mut report.errors);

    if candidates.is_empty() {
        report.errors.push(format!(
            "no supported sheet files found under {}",
            display_path(path)
        ));
        return report.finalize();
    }

    for candidate in candidates {
        let format = detect_sheet_format(&candidate);
        match decode_sheet(&candidate) {
            Ok(items) => {
                report.files.push(SheetFileReport::valid(
                    &candidate,
                    format.unwrap_or(SheetFormat::Invalid),
                    summarize_items(&items),
                ));
            }
            Err(err) => {
                report
                    .files
                    .push(SheetFileReport::invalid(&candidate, format, err.errors));
            }
        }
    }

    report.finalize()
}

pub fn render_plain_summary(report: &SheetVerificationReport) -> String {
    let mut lines = vec![format!(
        "verified {} sheet(s): {} valid, {} invalid",
        report.sheets_found, report.valid_sheets, report.invalid_sheets
    )];

    for error in &report.errors {
        lines.push(format!("error: {}", error));
    }

    for file in report.files.iter().filter(|file| !file.valid) {
        if let Some(first_error) = file.errors.first() {
            lines.push(format!("invalid {}: {}", file.path, first_error));
        } else {
            lines.push(format!("invalid {}", file.path));
        }
    }

    lines.join("\n")
}

pub fn render_plain_detail(report: &SheetVerificationReport) -> String {
    let mut lines = vec![format!(
        "verified {} sheet(s): {} valid, {} invalid",
        report.sheets_found, report.valid_sheets, report.invalid_sheets
    )];

    for error in &report.errors {
        lines.push(format!("error: {}", error));
    }

    for file in &report.files {
        lines.push(String::new());
        lines.push(format!("path: {}", file.path));
        lines.push(format!(
            "status: {}",
            if file.valid { "valid" } else { "invalid" }
        ));
        lines.push(format!(
            "format: {}",
            match file.format {
                Some(format) =>
                    serde_json::to_string(&format).unwrap_or_else(|_| "\"unknown\"".to_string()),
                None => "\"unknown\"".to_string(),
            }
            .trim_matches('"')
        ));

        if file.valid {
            lines.push(format!("rule_count: {}", file.rule_count.unwrap_or(0)));
            lines.push(format!(
                "pin_rule_count: {}",
                file.pin_rule_count.unwrap_or(0)
            ));
            lines.push(format!(
                "filter_rule_count: {}",
                file.filter_rule_count.unwrap_or(0)
            ));
        } else if file.errors.is_empty() {
            lines.push("errors: none".to_string());
        } else {
            lines.push("errors:".to_string());
            for error in &file.errors {
                lines.push(format!("  - {}", error));
            }
        }
    }

    lines.join("\n")
}

impl Filter {
    fn extend_with_items(&mut self, items: Vec<FilterItem>) -> &mut Self {
        wm_trace!("extend_with_items {:?}", items);
        let mut rules = items
            .into_iter()
            .map(|item| {
                let func = item.rule.gen_closure();
                ActionFuncPair {
                    action: item.action,
                    rule: item.rule,
                    func,
                }
            })
            .collect();
        self.rules.append(&mut rules);
        self
    }

    /// Extend filter with rules from `data` which can belong to any kind of `SheetFormat`
    pub fn extend_with<B: AsRef<[u8]>>(&mut self, data: B, format: SheetFormat) -> &mut Self {
        match decode_sheet_bytes(data.as_ref(), format) {
            Ok(items) => self.extend_with_items(items),
            Err(err) => {
                for error in err.errors {
                    wm_error!("{}", error);
                }
                self
            }
        }
    }

    /// Load sheets from disk at `path`
    /// sheet may be in any of three forms: unparsed form with ext .rule,
    /// two serialized forms: .json and .bin
    pub fn load_sheet<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let path = path.as_ref();

        if !path.exists() {
            wm_error!("{:?} does not exists", path);
            return self;
        }

        match decode_sheet(path) {
            Ok(items) => {
                self.extend_with_items(items);
            }
            Err(err) => {
                for error in err.errors {
                    wm_error!("{}", error);
                }
            }
        }

        self
    }

    /// Compile rule from disk file into json or bincode format
    pub fn compile<S: AsRef<Path>, P: AsRef<Path>>(rule: S, out: P) {
        let rule = rule.as_ref();
        let out = out.as_ref();
        wm_trace!("compile {:?} to {:?}", rule, out);

        if !rule.exists() {
            wm_error!("{:?} does not exists", rule);
            return;
        }

        if let Some(dir) = out.parent() {
            if !dir.exists() && create_dir_all(dir).is_err() {
                wm_error!("create {:?} failed", dir);
                return;
            }
        }

        let Some(output_format) = detect_sheet_format(out) else {
            wm_error!("compile failed: invalid extension");
            return;
        };

        let data = match fs::read(rule) {
            Ok(data) => data,
            Err(err) => {
                wm_error!("read {:?} failed: {}", rule, err);
                return;
            }
        };

        let items = match decode_sheet_bytes(&data, SheetFormat::Plain) {
            Ok(items) => items,
            Err(err) => {
                wm_error!("compile failed: {}", err.errors.join("; "));
                return;
            }
        };

        let mut dest = match File::create(out) {
            Err(err) => {
                wm_error!("create {:?} failed: {}", out, err);
                return;
            }
            Ok(file) => file,
        };

        let result = match output_format {
            SheetFormat::Json => {
                serde_json::to_writer(&mut dest, &items).map_err(|err| format!("json: {}", err))
            }
            SheetFormat::Binary => bincode::serialize(&items)
                .map_err(|err| format!("bin: {}", err))
                .and_then(|bytes| {
                    dest.write_all(&bytes)
                        .map_err(|err| format!("write: {}", err))
                }),
            _ => Err("invalid extension".to_string()),
        };

        if let Err(err) = result {
            wm_error!("compile failed: {}", err);
        } else {
            wm_trace!("compile done");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_sheet_format() {
        assert_eq!(detect_sheet_format("rules.rule"), Some(SheetFormat::Plain));
        assert_eq!(detect_sheet_format("rules.json"), Some(SheetFormat::Json));
        assert_eq!(detect_sheet_format("rules.bin"), Some(SheetFormat::Binary));
        assert_eq!(detect_sheet_format("rules.txt"), None);
    }

    #[test]
    fn test_extend_with_binary_data() {
        let items = vec![FilterItem {
            action: Action::Pin,
            rule: super::super::filter::FilterRule::ClientsOnly,
        }];
        let encoded = bincode::serialize(&items).unwrap();

        let mut filter = Filter::new();
        filter.extend_with(&encoded, SheetFormat::Binary);

        assert_eq!(filter.rule_count(), 1);
        assert_eq!(filter.pinned_rule_count(), 1);
    }

    #[test]
    fn test_verify_target_reports_recursive_mixed_results() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let nested = root.join("nested");
        fs::create_dir_all(&nested).unwrap();

        fs::write(root.join("good.rule"), "name = demo: pin").unwrap();
        fs::write(nested.join("bad.rule"), "attrs.map_state = broken").unwrap();
        fs::write(root.join("ignore.txt"), "ignored").unwrap();

        let report = verify_target(root);

        assert_eq!(report.sheets_found, 2);
        assert_eq!(report.valid_sheets, 1);
        assert_eq!(report.invalid_sheets, 1);
        assert!(!report.all_valid);
        assert_eq!(report.files[0].path, display_path(&root.join("good.rule")));
        assert_eq!(report.files[1].path, display_path(&nested.join("bad.rule")));
    }

    #[cfg(unix)]
    #[test]
    fn test_verify_target_skips_symlink_directories() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let root = dir.path();
        let actual = root.join("actual");
        fs::create_dir_all(&actual).unwrap();
        fs::write(actual.join("inside.rule"), "name = demo").unwrap();
        symlink(&actual, root.join("linked")).unwrap();

        let report = verify_target(root);

        assert_eq!(report.sheets_found, 1);
        assert_eq!(
            report.files[0].path,
            display_path(&actual.join("inside.rule"))
        );
    }
}
