//! lorem ipsum
#![crate_name = "varlink"]

extern crate itertools;

extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate bytes;
extern crate futures;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;


pub mod parser;
pub mod server;
