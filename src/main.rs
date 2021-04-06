#![feature(decl_macro)]
#![feature(proc_macro_hygiene)]

use std::thread;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};

use mysql::prelude::*;
use mysql::{OptsBuilder, Conn};

#[macro_use] extern crate rocket;

mod apiv1;
mod auth;
mod model;
mod ws_notifier;

const CONFIG_FILE: &str = "/etc/stomata/conf.toml";

fn main() {
    let mut conf = config::Config::default();
    conf.merge(config::File::with_name(CONFIG_FILE)).unwrap();
    let conf: HashMap<String, String> = conf.try_into().unwrap();

    let db_opts = OptsBuilder::new()
        .ip_or_hostname(Some(&conf["db_host"]))
        .user(Some(&conf["db_user"]))
        .pass(Some(&conf["db_pass"]));
    let mut db = Conn::new(db_opts).unwrap();

    db.query_drop(&format!("CREATE DATABASE IF NOT EXISTS {}", &conf["db_name"])).unwrap();
    db.query_drop(&format!("USE {}", &conf["db_name"])).unwrap();
    model::create_tables(&mut db);

    let db_conn = Arc::new(Mutex::new(db));

    let db = db_conn.clone();
    let http_server = thread::spawn(move || {
        apiv1::run(db, conf);
    });

    let db = db_conn.clone();
    let ws_server = thread::spawn(move || {
        ws_notifier::run(db);
    });

    http_server.join().unwrap();
    ws_server.join().unwrap();
}
