extern crate varlink_generator;

use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

#[cfg(unix)]
#[test]
fn test_generate() {
    fn compare_files(a: std::path::PathBuf, b: std::path::PathBuf) -> bool {
        let fa = File::open(a).unwrap();
        let fb = File::open(b).unwrap();
        fa.bytes()
            .zip(fb.bytes())
            .all(|(a, b)| a.unwrap() == b.unwrap())
    }

    std::env::set_var("OUT_DIR", std::env::temp_dir());
    varlink_generator::cargo_build("tests/org.example.complex.varlink");
    let mut path = std::path::PathBuf::from(std::env::temp_dir());
    path.push("org.example.complex.rs");

    assert!(Command::new("rustfmt")
        .arg(path.to_str().unwrap())
        .output()
        .is_ok());

    let path2 = path.with_extension("orig");

    assert!(std::fs::copy(
        std::path::PathBuf::from("tests/org.example.complex.rs_out"),
        path2.clone(),
    )
    .is_ok());

    assert!(Command::new("rustfmt")
        .arg(path2.clone().to_str().unwrap())
        .output()
        .is_ok());

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
