//! lorem ipsum
#![crate_name = "varlink"]

extern crate itertools;

extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate bytes;

pub mod parser;
pub mod server;
