//! lorem ipsum
#![crate_name = "varlink"]

extern crate itertools;

extern crate bytes;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

pub mod parser;
pub mod server;
