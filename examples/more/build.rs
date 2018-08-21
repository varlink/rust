extern crate varlink_generator;

fn main() {
    varlink_generator::cargo_build_tosource("src/org.example.more.varlink", true);
}
