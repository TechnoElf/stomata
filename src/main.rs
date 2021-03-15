#![feature(decl_macro)]
#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::http::Status;

#[get("/")]
fn index() -> &'static str {
    "Hello, Rocket!"
}

#[get("/dbg?<out>")]
fn dbg(out: Option<String>) -> Status {
    println!("[/dbg]: {:?}", out);
    Status::Ok
}

fn main() {
    rocket::ignite().mount("/", routes![index, dbg]).launch();
}
