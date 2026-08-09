extern crate varlink_generator;

use std::fs;
use std::io::prelude::*;
use std::process::Command;
use tempfile::TempDir;
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
            false
        } else {
            true
        }
    }

    let tmpdir = TempDir::with_prefix("test_generate").unwrap();

    std::env::set_var("OUT_DIR", tmpdir.path().as_os_str());
    varlink_generator::cargo_build("tests/org.example.complex.varlink");
    let mut path = tmpdir.path().to_owned();
    path.push("org.example.complex.rs");

    let status = Command::new("rustfmt")
        .arg("--edition=2018")
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
            path.to_str().unwrap(),
            path2.to_str().unwrap()
        );
    }
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path2);
}

// Regression: an `error` whose parameters contain anonymous nested types must
// emit each generated helper type exactly once. The error definitions are
// visited by two passes (the error-code/reply_* generation and the per-error
// `_Args` struct generation); a bug had both passes emit the anonymous helper
// types, producing duplicate `struct`/`enum` definitions that fail to compile.
#[test]
fn error_anonymous_types_emitted_once() {
    let src = "interface org.example.test\n\
               error Foo (bar: (a: int, b: string), baz: (x: bool))\n";
    let code = varlink_generator::compile(src.to_string())
        .unwrap()
        .to_string();

    for def in ["struct r#Foo_Args_bar", "struct r#Foo_Args_baz"] {
        let count = code.matches(def).count();
        assert_eq!(count, 1, "`{def}` was emitted {count} times, expected once");
    }
}
