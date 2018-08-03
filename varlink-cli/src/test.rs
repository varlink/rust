use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn test_exec() {
    let mut cmd = Command::main_binary().unwrap();
    cmd.assert().success();
}
