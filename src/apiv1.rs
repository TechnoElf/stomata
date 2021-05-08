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

use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::path::PathBuf;

use mysql::Pool;
use openapi::v3_0::*;
use rocket::{State, Request};
use rocket::http::Status;
use rocket::response::Redirect;
use rocket_contrib::json::Json;
use uuid::Uuid;

use crate::model::*;
use crate::auth::*;
use crate::ws_notifier::*;

type ApiResp<T> = Result<Json<T>, Status>;
type DbConn = Arc<Mutex<Pool>>;
type Conf = HashMap<String, String>;
type WsRequests = Arc<Mutex<Vec<WsRequest>>>;

#[get("/")]
fn index() -> Redirect {
    Redirect::to(uri!(root))
}

#[options("/<_path..>")]
fn options(_path: PathBuf) {}

#[get("/v1")]
fn root() -> Json<Spec> {
    Json(Spec {
        openapi: "3.1.0".to_string(),
        info: Info {
            title: "Thyme API / Stomata".to_string(),
            version: "0.1.0".to_string(),
            .. Default::default()
        },
        servers: Some(vec![Server {
            url: "https://stomata.undertheprinter.com/v1".to_string(),
            .. Default::default()
        }]),
        .. Default::default()
    })
}

#[post("/v1/stations", data = "<req>")]
fn stations_post(req: Json<StationsReq>, db: State<DbConn>) -> ApiResp<StationsResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    if get_station(req.id, &mut db).is_err() {
        let token = Uuid::new_v4().to_simple().encode_lower(&mut Uuid::encode_buffer()).to_string();
        let hash = BasicAuth::from_parts(&req.id.to_string(), &token).hash();
        add_station(req.id, &req.name, &hash, &mut db)?;
        Ok(Json(StationsResp {
            token
        }))
    } else {
        Err(Status::Conflict)
    }
}

#[get("/v1/stations/<id>")]
fn station_get(id: usize, db: State<DbConn>, auth: BasicAuth) -> ApiResp<StationResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let station = get_station(id, &mut db)?;
    if auth.verify(&station.token) || station.owner.as_ref().map(|o| Ok(auth.verify(&get_user(o, &mut db)?.pass))).unwrap_or(Ok(false))? {
        Ok(Json(StationResp {
            name: station.name,
            owner: station.owner
        }))
    } else {
        Err(Status::Unauthorized)
    }
}

#[put("/v1/stations/<id>", data = "<req>")]
fn station_put(id: usize, req: Json<StationReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let mut station = get_station(id, &mut db)?;
    if auth.verify(&station.token) || station.owner.as_ref().map(|o| Ok(auth.verify(&get_user(o, &mut db)?.pass))).unwrap_or(Ok(false))? {
        station.name = req.name.clone();
        update_station(station, &mut db)?;
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/v1/stations/<id>/data")]
fn data_get(id: usize, db: State<DbConn>, auth: BasicAuth) -> ApiResp<DataResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let station = get_station(id, &mut db)?;
    if station.owner.as_ref().map(|o| Ok(auth.verify(&get_user(o, &mut db)?.pass))).unwrap_or(Ok(false))? {
        let data = get_data(station.id, &mut db)?;
        Ok(Json(DataResp {
            data: data.into_iter().map(|d| DataElement {
                time: d.time,
                moisture: d.moisture,
                temperature: d.temperature,
                tank_empty: d.tank_empty
            }).collect()
        }))
    } else {
        Err(Status::Unauthorized)
    }
}

#[post("/v1/stations/<id>/data", data = "<req>")]
fn data_post(id: usize, req: Json<DataReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let station = get_station(id, &mut db)?;
    if auth.verify(&station.token) {
        add_data(station.id, req.moisture, req.temperature, req.tank_empty, &mut db)?;
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/v1/stations/<id>/state")]
fn state_get(id: usize, db: State<DbConn>, auth: BasicAuth) -> ApiResp<StateResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let station = get_station(id, &mut db)?;
    if auth.verify(&station.token) || station.owner.as_ref().map(|o| Ok(auth.verify(&get_user(o, &mut db)?.pass))).unwrap_or(Ok(false))? {
        Ok(Json(StateResp {
            state: station.state
        }))
    } else {
        Err(Status::Unauthorized)
    }
}

#[put("/v1/stations/<id>/state", data = "<req>")]
fn state_put(id: usize, req: Json<StateReq>, db: State<DbConn>, ws_reqs: State<WsRequests>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let mut station = get_station(id, &mut db)?;
    if auth.verify(&station.token) || station.owner.as_ref().map(|o| Ok(auth.verify(&get_user(o, &mut db)?.pass))).unwrap_or(Ok(false))? {
        let mut ws_reqs = ws_reqs.lock().or(Err(Status::InternalServerError))?;
        ws_reqs.push(WsRequest::UpdateState(WsUpdateState {
            id: station.id,
            state: req.state.clone()
        }));

        station.state = req.state.clone();
        update_station(station, &mut db)?;

        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[post("/v1/users", data = "<req>")]
fn users_post(req: Json<UsersReq>, db: State<DbConn>) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let hash = BasicAuth::from_parts(&req.login, &req.pass).hash();
    add_user(&req.login, &req.name, &hash, &mut db)?;
    Ok(Json(EmptyResp {}))
}

#[get("/v1/users/<login>")]
fn user_get(login: String, db: State<DbConn>, auth: BasicAuth) -> ApiResp<UserResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let user = get_user(&login, &mut db)?;
    if auth.verify(&user.pass) {
        Ok(Json(UserResp {
            name: user.name
        }))
    } else {
        Err(Status::Unauthorized)
    }
}

#[put("/v1/users/<login>", data = "<req>")]
fn user_put(login: String, req: Json<UserReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let mut user = get_user(&login, &mut db)?;
    if auth.verify(&user.pass) {
        let pass_hash = BasicAuth::from_parts(&user.login, &req.pass).hash();
        user.pass = pass_hash;
        user.name = req.name.clone();
        update_user(user, &mut db)?;
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/v1/users/<login>/stations")]
fn user_stations_get(login: String, db: State<DbConn>, auth: BasicAuth) -> ApiResp<UserStationsResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let user = get_user(&login, &mut db)?;
    if auth.verify(&user.pass) {
        let stations = get_stations(&login, &mut db)?;
        Ok(Json(UserStationsResp {
            stations: stations.into_iter().map(|s| s.id).collect()
        }))
    } else {
        Err(Status::Unauthorized)
    }
}

#[post("/v1/users/<login>/stations", data = "<req>")]
fn user_stations_post(login: String, req: Json<UserStationsReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?.get_conn().or(Err(Status::InternalServerError))?;
    let user = get_user(&login, &mut db)?;
    if auth.verify(&user.pass) {
        let mut station = get_station(req.id, &mut db)?;
        if station.owner.is_none() {
            station.owner = Some(user.login);
            update_station(station, &mut db)?;
        }
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[catch(400)] 
fn bad_request(_req: &Request) {}

#[catch(401)] 
fn unauthorised(_req: &Request) {}

#[catch(404)] 
fn not_found(_req: &Request) {}

#[catch(409)] 
fn conflict(_req: &Request) {}

#[catch(422)] 
fn unprocessable(_req: &Request) {}

#[catch(500)] 
fn server_error(_req: &Request) {}

pub fn run(db_conn: DbConn, conf: Conf, ws_reqs: WsRequests) {
    rocket::ignite()
        .mount("/", routes![index, options, root, stations_post, station_get, station_put, data_get, data_post, state_get, state_put, users_post, user_get, user_put, user_stations_get, user_stations_post])
        .register(catchers![bad_request, unauthorised, not_found, conflict, unprocessable, server_error])
        .manage(db_conn).manage(conf).manage(ws_reqs).launch();
}
