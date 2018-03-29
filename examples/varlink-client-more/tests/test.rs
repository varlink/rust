extern crate varlink;

use std::process::Command;

#[test]
fn test_exec() {
    let output = Command::new("../../target/debug/varlink-client-more")
        .arg("exec:../../target/debug/varlink-server-more")
        .output()
        .expect("failed to execute process");
    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());
}
