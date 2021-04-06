use std::time::SystemTime;

use mysql::Conn;
use mysql::prelude::Queryable;
use rocket::http::Status;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct UserRow {
    pub name: String,
    pub pass: String
}

#[derive(Debug)]
pub struct StationRow {
    pub id: usize,
    pub name: String,
    pub state: String,
    pub owner: Option<String>,
    pub token: String
}

#[derive(Debug)]
pub struct DataRow {
    pub station: usize,
    pub time: usize,
    pub val: isize
}

pub fn create_tables(db: &mut Conn) {
    db.query_drop("CREATE TABLE IF NOT EXISTS users (name TEXT NOT NULL, pass TEXT NOT NULL)").unwrap();
    db.query_drop("CREATE TABLE IF NOT EXISTS stations (id INT NOT NULL, name TEXT NOT NULL, state TEXT NOT NULL, owner TEXT, token TEXT NOT NULL)").unwrap();
    db.query_drop("CREATE TABLE IF NOT EXISTS data (station INT NOT NULL, time INT NOT NULL, val INT NOT NULL)").unwrap();
}

pub fn get_user(name: &str, db: &mut Conn) -> Result<UserRow, Status> {
    Ok(db.exec_first("SELECT * FROM users WHERE name = ?", (name,)).or(Err(Status::InternalServerError))?
        .map(|(name, pass)| UserRow { name, pass }).ok_or(Status::NotFound)?)
}

pub fn get_station(id: usize, db: &mut Conn) -> Result<StationRow, Status> {
    Ok(db.exec_first("SELECT * FROM stations WHERE id = ?", (id,)).or(Err(Status::InternalServerError))?
        .map(|(id, name, state, owner, token)| StationRow { id, name, state, owner, token }).ok_or(Status::NotFound)?)
}

pub fn get_stations(owner: &str, db: &mut Conn) -> Result<Vec<StationRow>, Status> {
    Ok(db.exec("SELECT * FROM stations WHERE owner = ?", (owner,)).or(Err(Status::InternalServerError))?
        .into_iter().map(|(id, name, state, owner, token)| StationRow { id, name, state, owner, token }).collect())
}

pub fn get_data(station: usize, db: &mut Conn) -> Result<Vec<DataRow>, Status> {
    Ok(db.exec("SELECT * FROM data WHERE station = ?", (station,)).or(Err(Status::InternalServerError))?
        .into_iter().map(|(station, time, val)| DataRow { station, time, val }).collect())
}

pub fn update_user(user: UserRow, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("UPDATE users SET pass = ? WHERE name = ?", (&user.pass, &user.name)).or(Err(Status::InternalServerError))?)
}

pub fn update_station(station: StationRow, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("UPDATE stations SET name = ?, state = ?, owner = ?, token = ? WHERE id = ?", (&station.name, &station.state, &station.owner, &station.token, station.id)).or(Err(Status::InternalServerError))?)
}

pub fn add_user(name: &str, pass: &str, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("INSERT INTO users (name, pass) VALUES (?, ?)", (name, pass)).or(Err(Status::InternalServerError))?)
}

pub fn add_station(id: usize, name: &str, token: &str, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("INSERT INTO stations (id, name, state, token) VALUES (?, ?, ?, ?)", (id, name, "idle", token)).or(Err(Status::InternalServerError))?)
}

pub fn add_data(station: usize, val: isize, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("INSERT INTO data (station, time, val) VALUES (?, ?, ?)", (station, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(), val)).or(Err(Status::InternalServerError))?)
}

#[derive(Debug, Deserialize)]
pub struct StationsReq {
    pub id: usize,
    pub name: String
}

#[derive(Debug, Deserialize)]
pub struct StationReq {
    pub name: String
}

#[derive(Debug, Deserialize)]
pub struct DataReq {
    pub val: isize
}

#[derive(Debug, Deserialize)]
pub struct StateReq {
    pub state: String
}

#[derive(Debug, Deserialize)]
pub struct UsersReq {
    pub name: String,
    pub pass: String
}

#[derive(Debug, Deserialize)]
pub struct UserReq {
    pub pass: String
}

#[derive(Debug, Deserialize)]
pub struct UserStationsReq {
    pub id: usize
}

#[derive(Debug, Serialize)]
pub struct EmptyResp {}

#[derive(Debug, Serialize)]
pub struct StationsResp {
    pub token: String
}

#[derive(Debug, Serialize)]
pub struct StationResp {
    pub name: String,
    pub owner: Option<String>
}

#[derive(Debug, Serialize)]
pub struct DataResp {
    pub data: Vec<DataElement>
}

#[derive(Debug, Serialize)]
pub struct DataElement {
    pub time: usize,
    pub val: isize
}

#[derive(Debug, Serialize)]
pub struct StateResp {
    pub state: String
}

#[derive(Debug, Serialize)]
pub struct UserResp {}

#[derive(Debug, Serialize)]
pub struct UserStationsResp {
    pub stations: Vec<usize>
}
