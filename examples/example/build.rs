extern crate varlink_generator;

fn main() {
    varlink_generator::cargo_build_tosource("src/io.systemd.network.varlink", true);

    /*
    varlink::generator::cargo_build_tosource_options(
        "src/io.systemd.network.varlink",
        true,
        &varlink::generator::GeneratorOptions {
            int_type: Some("i128"),
            ..Default::default()
        },
    );
    */
}
