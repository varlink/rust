extern crate varlink;

fn main() {
    varlink::generator::cargo_build("src/io.systemd.network.varlink");
}
