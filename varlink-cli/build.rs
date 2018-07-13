extern crate varlink;

fn main() {
    varlink::generator::cargo_build_tosource("src/org.varlink.resolver.varlink", false);
}
