extern crate varlink_generator;

use std::fs;
use std::io::prelude::*;
use std::process::Command;
use tempdir::TempDir;
use unified_diff::diff;

#[cfg(unix)]
#[test]
fn test_generate() {
    fn compare_files(a: std::path::PathBuf, b: std::path::PathBuf) -> bool {
        let from_content = fs::read(a.clone()).unwrap();
        let to_content = fs::read(b.clone()).unwrap();

        if !from_content
            .bytes()
            .zip(to_content.bytes())
            .all(|(a, b)| a.unwrap() == b.unwrap())
        {
            std::io::stdout()
                .write_all(&diff(
                    &from_content,
                    &a.to_string_lossy(),
                    &to_content,
                    &b.to_string_lossy(),
                    1,
                ))
                .unwrap();
            return false;
        } else {
            return true;
        }
    }

    let tmpdir = TempDir::new("test_generate").unwrap();

    std::env::set_var("OUT_DIR", tmpdir.path().as_os_str());
    varlink_generator::cargo_build("tests/org.example.complex.varlink");
    let mut path = tmpdir.path().to_owned();
    path.push("org.example.complex.rs");

    let status = Command::new("rustfmt")
        .arg("--emit")
        .arg("files")
        .arg(path.to_str().unwrap())
        .status()
        .unwrap();

    assert!(status.success());

    let path2 = path.with_extension("orig");

    assert!(std::fs::copy(
        std::path::PathBuf::from("tests/org.example.complex.rs_out"),
        path2.clone(),
    )
    .is_ok());

    let status = Command::new("rustfmt")
        .arg("--emit")
        .arg("files")
        .arg(path2.to_str().unwrap())
        .status()
        .unwrap();

    assert!(status.success());

    if !compare_files(path2.clone(), path.clone()) {
        panic!(
            "Generated files differ: '{}' versus '{}'",
            &path.to_str().unwrap(),
            &path2.to_str().unwrap()
        );
    }
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path2);
}
