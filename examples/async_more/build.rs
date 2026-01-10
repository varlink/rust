fn main() {
    varlink_generator::cargo_build_tosource_options(
        "src/org.example.more.varlink",
        true,
        &varlink_generator::GeneratorOptions {
            generate_async: true,
            ..Default::default()
        },
    );
}
