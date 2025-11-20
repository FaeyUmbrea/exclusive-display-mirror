pub fn set_test_mode(cmd: &mut assert_cmd::Command) {
    cmd.env("BOOTSTRAPPER_TEST_MODE", "1");
}
