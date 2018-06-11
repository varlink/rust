all: src/varlink_grammar.rs 

src/varlink_grammar.rs: src/varlink_grammar.rustpeg
	@rust-peg $<  > $@

check: src/varlink_grammar.rs
	@cargo test


