extern crate varlink_generator;

use std::env;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    // Generate sync versions
    varlink_generator::cargo_build("src/org.varlink.resolver.varlink");

    // Generate async versions when tokio feature is enabled
    if env::var("CARGO_FEATURE_TOKIO").is_ok() {
        // Generate async version for org.varlink.resolver
        generate_async(
            "src/org.varlink.resolver.varlink",
            "org_varlink_resolver_async.rs",
        );
    }
}

fn generate_async(varlink_file: &str, output_file: &str) {
    let out_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    let output_path = out_dir.join(output_file);

    // Open input and output files
    let mut input = File::open(varlink_file)
        .unwrap_or_else(|e| panic!("Could not open {}: {}", varlink_file, e));

    let mut output = File::create(&output_path)
        .unwrap_or_else(|e| panic!("Could not create {}: {}", output_path.display(), e));

    // Generate async code using generate_with_options with tosource=false
    // tosource=false is critical - it prevents generating #![...] inner attributes
    // which are not allowed when code is included via include!() macro
    varlink_generator::generate_with_options(
        &mut input,
        &mut output,
        &varlink_generator::GeneratorOptions {
            generate_async: true,
            ..Default::default()
        },
        false, // tosource: false for include!() usage
    )
    .unwrap_or_else(|e| panic!("Could not generate async code for {}: {}", varlink_file, e));

    println!("cargo:rerun-if-changed={}", varlink_file);
}
