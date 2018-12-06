extern crate varlink_generator;

use std::fs::File;
use std::io::prelude::*;
// use std::process::Command;

fn compare_files(a: std::path::PathBuf, b: std::path::PathBuf) -> bool {
    let fa = File::open(a).unwrap();
    let fb = File::open(b).unwrap();
    fa.bytes()
        .zip(fb.bytes())
        .all(|(a, b)| a.unwrap() == b.unwrap())
}

#[cfg(unix)]
#[test]
fn test_generate() {
    std::env::set_var("OUT_DIR", std::env::temp_dir());
    varlink_generator::cargo_build("tests/org.example.complex.varlink");
    let mut path = std::path::PathBuf::from(std::env::temp_dir());
    path.push("org.example.complex.rs");

    // assert!(
    // Command::new("rustfmt")
    // .arg(path.to_str().unwrap())
    // .output()
    // .is_ok()
    // );
    //
    if !compare_files(
        std::path::PathBuf::from("tests/org.example.complex.rs_out"),
        path.clone(),
    ) {
        panic!(
            "Generated files differ: '{}' versus '{}'",
            "tests/org.example.complex.rs_out",
            &path.to_str().unwrap()
        );
    }
    let _ = std::fs::remove_file(path);
}
