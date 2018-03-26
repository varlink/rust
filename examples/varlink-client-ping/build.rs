extern crate varlink;

fn main() {
    varlink::generator::cargo_build_tosource("src/org.example.ping.varlink", true);
}
