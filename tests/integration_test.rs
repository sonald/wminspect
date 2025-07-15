#[cfg(test)]
mod integration_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use tempfile::tempdir;

    #[test]
    fn test_cli_flags() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--only-mapped")
            .assert()
            .success()
            .stdout(predicate::str::contains("Application started"));
    }

    #[test]
    fn test_x11_event_playback() {
        // X11 event playback test using x11rb's stub
    }
}
