use super::filter::parse_rule_text_with_diagnostics;
use super::sheets::{SheetSummary, summarize_items};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinSheetPreset {
    pub name: &'static str,
    pub summary: &'static str,
    pub rule_text: &'static str,
}

const BUILTIN_PRESETS: [BuiltinSheetPreset; 5] = [
    BuiltinSheetPreset {
        name: "mapped-clients",
        summary: "Only WM-managed, viewable client windows",
        rule_text: include_str!("../../sheets/mapped-clients.rule"),
    },
    BuiltinSheetPreset {
        name: "clean-monitor",
        summary: "Low-noise default view for troubleshooting",
        rule_text: include_str!("../../sheets/clean-monitor.rule"),
    },
    BuiltinSheetPreset {
        name: "large-windows",
        summary: "Focus on larger primary work windows",
        rule_text: include_str!("../../sheets/large-windows.rule"),
    },
    BuiltinSheetPreset {
        name: "special-windows",
        summary: "Inspect docks, panels, popups, and helper windows",
        rule_text: include_str!("../../sheets/special-windows.rule"),
    },
    BuiltinSheetPreset {
        name: "hidden-or-unviewable",
        summary: "Inspect hidden or non-viewable windows",
        rule_text: include_str!("../../sheets/hidden-or-unviewable.rule"),
    },
];

pub fn builtin_sheet_presets() -> &'static [BuiltinSheetPreset] {
    &BUILTIN_PRESETS
}

pub fn builtin_sheet_preset(name: &str) -> Option<&'static BuiltinSheetPreset> {
    BUILTIN_PRESETS.iter().find(|preset| preset.name == name)
}

pub fn builtin_sheet_preset_summary(name: &str) -> Option<SheetSummary> {
    let preset = builtin_sheet_preset(name)?;
    let items = parse_rule_text_with_diagnostics(preset.rule_text).ok()?;
    Some(summarize_items(&items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_builtin_presets_parse_cleanly() {
        for preset in builtin_sheet_presets() {
            let items = parse_rule_text_with_diagnostics(preset.rule_text)
                .unwrap_or_else(|errors| panic!("{} failed to parse: {:?}", preset.name, errors));
            assert!(
                !items.is_empty(),
                "{} should contain at least one rule",
                preset.name
            );
        }
    }

    #[test]
    fn test_builtin_preset_names_are_unique_and_ordered() {
        let presets = builtin_sheet_presets();
        let names: Vec<_> = presets.iter().map(|preset| preset.name).collect();
        let unique: HashSet<_> = names.iter().copied().collect();

        assert_eq!(names.len(), unique.len());
        assert_eq!(
            names,
            vec![
                "mapped-clients",
                "clean-monitor",
                "large-windows",
                "special-windows",
                "hidden-or-unviewable"
            ]
        );
    }

    #[test]
    fn test_builtin_preset_summary_counts() {
        let clean = builtin_sheet_preset_summary("clean-monitor").unwrap();
        assert_eq!(clean.rule_count, 5);
        assert_eq!(clean.filter_rule_count, 5);
        assert_eq!(clean.pin_rule_count, 0);

        let special = builtin_sheet_preset_summary("special-windows").unwrap();
        assert_eq!(special.rule_count, 1);
        assert_eq!(special.filter_rule_count, 1);
        assert_eq!(special.pin_rule_count, 0);
    }

    #[test]
    fn test_builtin_registry_matches_official_sheet_files() {
        let sheets_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/sheets");
        let mut names = std::fs::read_dir(sheets_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rule"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        names.sort();

        let mut registry = builtin_sheet_presets()
            .iter()
            .map(|preset| preset.name.to_string())
            .collect::<Vec<_>>();
        registry.sort();

        assert_eq!(registry, names);
    }
}
