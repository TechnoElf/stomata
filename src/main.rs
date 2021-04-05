#![feature(decl_macro)]
#![feature(proc_macro_hygiene)]

use std::thread;
use std::sync::{Mutex, Arc};

use mysql::prelude::*;
use mysql::{OptsBuilder, Conn};

#[macro_use] extern crate rocket;

mod ws_notifier;
mod apiv1;
mod model;
mod auth;

fn main() {
    let db_opts = OptsBuilder::new()
        .ip_or_hostname(Some("vacuole"))
        .user(Some(""))
        .pass(Some(""));
    let db_conn = Arc::new(Mutex::new(Conn::new(db_opts).unwrap()));

    {
        let mut db = db_conn.lock().unwrap();
        db.query_drop("CREATE DATABASE IF NOT EXISTS vacuole").unwrap();
        db.query_drop("USE vacuole").unwrap();
        model::create_tables(&mut db);
    }

    let db = db_conn.clone();
    let http_server = thread::spawn(move || {
        apiv1::run(db);
    });

    let db = db_conn.clone();
    let ws_server = thread::spawn(move || {
        ws_notifier::run(db);
    });

    http_server.join().unwrap();
    ws_server.join().unwrap();
}
