use std::process::Command;
use assert_cmd::prelude::*;

#[test]
fn test_exec() {
    let mut cmd = Command::main_binary().unwrap();
    cmd.assert().success();
}