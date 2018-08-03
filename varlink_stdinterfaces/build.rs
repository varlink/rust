extern crate varlink;

fn main() {
    varlink::generator::cargo_build("src/org.varlink.resolver.varlink");
    varlink::generator::cargo_build("src/org.varlink.service.varlink");
}
