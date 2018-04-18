extern crate varlink;

fn main() {
    varlink::generator::cargo_build_tosource("src/io.systemd.network.varlink", true);
}
