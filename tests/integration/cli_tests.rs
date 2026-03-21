use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[cfg(test)]
mod cli_integration_tests {
    use super::*;

    #[test]
    fn test_help_command() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage: wminspect"));
    }

    #[test]
    fn test_version_command() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_show_grammar_flag() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--show-grammar")
            .assert()
            .success()
            .stdout(predicate::str::contains("grammar:"));
    }

    #[test]
    fn test_cli_flag_matrix() {
        let flags = vec![
            "--only-mapped",
            "--colored",
            "--omit-hidden",
            "--no-override-redirect",
            "--no-special",
            "--diff",
            "--clients-only",
        ];

        // Test individual flags
        for flag in &flags {
            let mut cmd = Command::cargo_bin("wminspect").unwrap();
            cmd.arg(flag).assert().success();
        }

        // Test combination of flags
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["--only-mapped", "--colored", "--omit-hidden"])
            .assert()
            .success();
    }

    #[test]
    fn test_filter_flag() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["--filter", "name = test"]).assert().success();
    }

    #[test]
    fn test_monitor_flag() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.env("WMINSPECT_MONITOR_ONCE", "1")
            .arg("--monitor")
            .assert()
            .success();
    }

    #[test]
    fn test_monitor_subcommand() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.env("WMINSPECT_MONITOR_ONCE", "1")
            .arg("monitor")
            .assert()
            .success();
    }

    #[test]
    fn test_sheet_compile_subcommand() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("test.rule");
        let json_file = temp_dir.path().join("test.json");

        // Create a test rule file
        fs::write(&rule_file, "name = test: pin").unwrap();

        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&[
            "sheet",
            "--compile",
            rule_file.to_str().unwrap(),
            json_file.to_str().unwrap(),
        ])
        .assert()
        .success();

        // Check that the JSON file was created
        assert!(json_file.exists());
    }

    #[test]
    fn test_sheet_load_subcommand() {
        let temp_dir = tempdir().unwrap();
        let json_file = temp_dir.path().join("test.json");

        // Create a test JSON file
        fs::write(&json_file, r#"[{"action":"Pin","rule":"ClientsOnly"}]"#).unwrap();

        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["sheet", "--load", json_file.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn test_sheet_verify_file_summary() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("valid.rule");
        fs::write(&rule_file, "name = test: pin").unwrap();

        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["sheet", "verify", rule_file.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "verified 1 sheet(s): 1 valid, 0 invalid",
            ))
            .stdout(predicate::str::contains("0:").not());
    }

    #[test]
    fn test_sheet_verify_file_detail() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("valid.rule");
        fs::write(&rule_file, "name = test: pin").unwrap();

        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["sheet", "verify", rule_file.to_str().unwrap(), "--detail"])
            .assert()
            .success()
            .stdout(predicate::str::contains("rule_count: 1"))
            .stdout(predicate::str::contains("pin_rule_count: 1"));
    }

    #[test]
    fn test_sheet_verify_file_json() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("valid.rule");
        fs::write(&rule_file, "name = test: pin").unwrap();

        let output = Command::cargo_bin("wminspect")
            .unwrap()
            .args(&["sheet", "verify", rule_file.to_str().unwrap(), "--json"])
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let json: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(json["sheets_found"], 1);
        assert_eq!(json["all_valid"], true);
        assert_eq!(json["files"][0]["format"], "rule");
    }

    #[test]
    fn test_sheet_builtin_list() {
        Command::cargo_bin("wminspect")
            .unwrap()
            .env_remove("DISPLAY")
            .args(&["sheet", "builtin", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("mapped-clients"))
            .stdout(predicate::str::contains("clean-monitor"))
            .stdout(predicate::str::contains("hidden-or-unviewable"));
    }

    #[test]
    fn test_sheet_builtin_show() {
        Command::cargo_bin("wminspect")
            .unwrap()
            .env_remove("DISPLAY")
            .args(&["sheet", "builtin", "show", "clean-monitor"])
            .assert()
            .success()
            .stdout(predicate::str::contains("clients: filter;"))
            .stdout(predicate::str::contains("name=*notification*"));
    }

    #[test]
    fn test_sheet_builtin_show_invalid_name() {
        Command::cargo_bin("wminspect")
            .unwrap()
            .env_remove("DISPLAY")
            .args(&["sheet", "builtin", "show", "does-not-exist"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown built-in preset"))
            .stderr(predicate::str::contains("mapped-clients"));
    }

    #[test]
    fn test_sheet_verify_directory_recursive_mixed_results() {
        let temp_dir = tempdir().unwrap();
        let nested = temp_dir.path().join("nested");
        fs::create_dir_all(&nested).unwrap();

        let valid_rule = temp_dir.path().join("valid.rule");
        fs::write(&valid_rule, "name = ok: pin").unwrap();

        let valid_bin = nested.join("valid.bin");
        let compiled = serde_json::from_str::<Vec<wminspect::dsl::filter::FilterItem>>(
            r#"[{"action":"Pin","rule":"ClientsOnly"}]"#,
        )
        .unwrap();
        fs::write(&valid_bin, bincode::serialize(&compiled).unwrap()).unwrap();

        let invalid_rule = nested.join("invalid.rule");
        fs::write(&invalid_rule, "attrs.map_state = broken").unwrap();

        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["sheet", "verify", temp_dir.path().to_str().unwrap()])
            .assert()
            .failure()
            .stdout(predicate::str::contains(
                "verified 3 sheet(s): 2 valid, 1 invalid",
            ))
            .stdout(predicate::str::contains("invalid"));
    }

    #[test]
    fn test_sheet_verify_missing_path() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["sheet", "verify", "./does-not-exist"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("target does not exist"));
    }

    #[test]
    fn test_sheet_verify_unsupported_file() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("unsupported.txt");
        fs::write(&path, "name = test").unwrap();

        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["sheet", "verify", path.to_str().unwrap()])
            .assert()
            .failure()
            .stdout(predicate::str::contains("unsupported sheet format"));
    }

    #[test]
    fn test_sheet_commands_do_not_require_display() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("valid.rule");
        let json_file = temp_dir.path().join("valid.json");
        fs::write(&rule_file, "name = test: pin").unwrap();

        Command::cargo_bin("wminspect")
            .unwrap()
            .env_remove("DISPLAY")
            .args(&[
                "sheet",
                "--compile",
                rule_file.to_str().unwrap(),
                json_file.to_str().unwrap(),
            ])
            .assert()
            .success();

        Command::cargo_bin("wminspect")
            .unwrap()
            .env_remove("DISPLAY")
            .args(&["sheet", "verify", rule_file.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "verified 1 sheet(s): 1 valid, 0 invalid",
            ));
    }

    #[test]
    fn test_builtin_sheet_directory_verifies_cleanly() {
        Command::cargo_bin("wminspect")
            .unwrap()
            .env_remove("DISPLAY")
            .args(&["sheet", "verify", "./sheets"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "verified 5 sheet(s): 5 valid, 0 invalid",
            ));
    }

    #[test]
    fn test_preset_flag_can_be_combined_with_filter() {
        Command::cargo_bin("wminspect")
            .unwrap()
            .args(&["--preset", "mapped-clients", "--filter", "geom.width>=400"])
            .assert()
            .success();
    }

    #[test]
    fn test_preset_flag_invalid_name() {
        Command::cargo_bin("wminspect")
            .unwrap()
            .env_remove("DISPLAY")
            .args(&["--preset", "does-not-exist"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown built-in preset"))
            .stderr(predicate::str::contains("mapped-clients"));
    }

    #[test]
    fn test_invalid_filter_expression() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["--filter", "invalid syntax here"])
            .assert()
            .success(); // Should not crash even with invalid syntax
    }

    #[test]
    fn test_complex_filter_expression() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&[
            "--filter",
            "any(name = test*, all(geom.width > 400, geom.height > 300)): pin",
        ])
        .assert()
        .success();
    }
}
