#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

mod chart;

#[cfg(not(test))]
fn main() {
    chart::serve_web();
}

