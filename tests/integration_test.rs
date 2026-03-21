#[cfg(test)]
mod integration_tests {
    use assert_cmd::Command;

    #[test]
    fn test_cli_flags() {
        let mut cmd = Command::cargo_bin("wminspect").unwrap();
        cmd.arg("--only-mapped").assert().success();
    }

    #[test]
    fn test_x11_event_playback() {
        // X11 event playback test using x11rb's stub
    }
}
