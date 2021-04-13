/*
 * stomata - Backend for the Thyme project
 * Copyright (C) 2021 TechnoElf
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::time::SystemTime;

use mysql::Conn;
use mysql::prelude::Queryable;
use rocket::http::Status;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct UserRow {
    pub login: String,
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
    pub moisture: isize,
    pub temperature: f32,
    pub tank_empty: bool
}

pub fn create_tables(db: &mut Conn) {
    db.query_drop("CREATE TABLE IF NOT EXISTS users (login TEXT NOT NULL, name TEXT NOT NULL, pass TEXT NOT NULL)").unwrap();
    db.query_drop("CREATE TABLE IF NOT EXISTS stations (id INT NOT NULL, name TEXT NOT NULL, state TEXT NOT NULL, owner TEXT, token TEXT NOT NULL)").unwrap();
    db.query_drop("CREATE TABLE IF NOT EXISTS data (station INT NOT NULL, time INT NOT NULL, moisture INT NOT NULL, temperature FLOAT NOT NULL, tank_empty BOOL NOT NULL)").unwrap();
}

pub fn get_user(login: &str, db: &mut Conn) -> Result<UserRow, Status> {
    Ok(db.exec_first("SELECT * FROM users WHERE login = ?", (login,)).or(Err(Status::InternalServerError))?
        .map(|(login, name, pass)| UserRow { login, name, pass }).ok_or(Status::NotFound)?)
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
        .into_iter().map(|(station, time, moisture, temperature, tank_empty)| DataRow { station, time, moisture, temperature, tank_empty }).collect())
}

pub fn update_user(user: UserRow, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("UPDATE users SET name = ?, pass = ? WHERE login = ?", (&user.name, &user.pass, &user.login)).or(Err(Status::InternalServerError))?)
}

pub fn update_station(station: StationRow, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("UPDATE stations SET name = ?, state = ?, owner = ?, token = ? WHERE id = ?", (&station.name, &station.state, &station.owner, &station.token, station.id)).or(Err(Status::InternalServerError))?)
}

pub fn add_user(login: &str, name: &str, pass: &str, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("INSERT INTO users (login, name, pass) VALUES (?, ?, ?)", (login, name, pass)).or(Err(Status::InternalServerError))?)
}

pub fn add_station(id: usize, name: &str, token: &str, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("INSERT INTO stations (id, name, state, token) VALUES (?, ?, ?, ?)", (id, name, "idle", token)).or(Err(Status::InternalServerError))?)
}

pub fn add_data(station: usize, moisture: isize, temperature: f32, tank_empty: bool, db: &mut Conn) -> Result<(), Status> {
    Ok(db.exec_drop("INSERT INTO data (station, time, moisture, temperature, tank_empty) VALUES (?, ?, ?, ?, ?)", (station, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(), moisture, temperature, tank_empty)).or(Err(Status::InternalServerError))?)
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
    pub moisture: isize,
    pub temperature: f32,
    pub tank_empty: bool
}

#[derive(Debug, Deserialize)]
pub struct StateReq {
    pub state: String
}

#[derive(Debug, Deserialize)]
pub struct UsersReq {
    pub login: String,
    pub name: String,
    pub pass: String
}

#[derive(Debug, Deserialize)]
pub struct UserReq {
    pub name: String,
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
    pub moisture: isize,
    pub temperature: f32,
    pub tank_empty: bool
}

#[derive(Debug, Serialize)]
pub struct StateResp {
    pub state: String
}

#[derive(Debug, Serialize)]
pub struct UserResp {
    pub name: String
}

#[derive(Debug, Serialize)]
pub struct UserStationsResp {
    pub stations: Vec<usize>
}
