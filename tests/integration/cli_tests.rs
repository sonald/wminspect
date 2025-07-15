use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
use std::fs;

#[cfg(test)]
mod cli_integration_tests {
    use super::*;

    #[test]
    fn test_help_command() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("window manager inspector"));
    }

    #[test]
    fn test_version_command() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains("wminspect 0.3.0"));
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
            cmd.arg(flag)
                .assert()
                .success();
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
        cmd.args(&["--filter", "name = test"])
            .assert()
            .success();
    }

    #[test]
    fn test_monitor_flag() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--monitor")
            .assert()
            .success();
    }

    #[test]
    fn test_monitor_subcommand() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("monitor")
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
        cmd.args(&["sheet", "--compile", rule_file.to_str().unwrap(), json_file.to_str().unwrap()])
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
    fn test_invalid_filter_expression() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["--filter", "invalid syntax here"])
            .assert()
            .success(); // Should not crash even with invalid syntax
    }

    #[test]
    fn test_complex_filter_expression() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.args(&["--filter", "any(name = test*, all(geom.width > 400, geom.height > 300)): pin"])
            .assert()
            .success();
    }
}
