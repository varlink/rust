#![doc(
    html_logo_url = "https://varlink.org/images/varlink.png",
    html_favicon_url = "https://varlink.org/images/varlink-small.png"
)]
#![allow(unused_imports)]

extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

pub mod org_varlink_resolver;
pub mod org_varlink_service;
