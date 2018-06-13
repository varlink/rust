#[cfg(feature = "dynamic_peg")]
extern crate peg;

fn main() {
    #[cfg(feature = "dynamic_peg")]
    peg::cargo_build("src/varlink_grammar.rustpeg");
}
