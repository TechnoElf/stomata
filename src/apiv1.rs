use std::sync::{Mutex, Arc};

use mysql::Conn;

use velcro::btree_map;

use rocket::{State, Request};
use rocket::response::{Redirect, status};
use rocket::http::Status;
use rocket_contrib::json::Json;

use openapi::v3_0::*;

use uuid::Uuid;

use crate::model::*;
use crate::auth::*;

type ApiResp<T> = Result<Json<T>, Status>;
type DbConn = Arc<Mutex<Conn>>;

#[get("/")]
fn index() -> Redirect {
    Redirect::to(uri!(root))
}

#[get("/v1")]
fn root() -> status::Custom<Json<Spec>> {
    status::Custom(Status::ImATeapot, Json(Spec {
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
        paths: btree_map!{
            "/stations".to_string(): PathItem {
                post: Some(Operation {
                    summary: Some("Register a station (station side)".to_string()),
                    .. Default::default()
                }),
                .. Default::default()
            },
            "/stations/{id}".to_string(): PathItem {
                get: Some(Operation {
                    summary: Some("Retrieve station data".to_string()),
                    .. Default::default()
                }),
                put: Some(Operation {
                    summary: Some("Modify station data".to_string()),
                    .. Default::default()
                }),
                .. Default::default()
            },
            "/stations/{id}/data".to_string(): PathItem {
                get: Some(Operation {
                    summary: Some("Retrieve sensor data".to_string()),
                    .. Default::default()
                }),
                post: Some(Operation {
                    summary: Some("Record sensor data".to_string()),
                    .. Default::default()
                }),
                .. Default::default()
            },
            "/stations/{id}/status".to_string(): PathItem {
                get: Some(Operation {
                    summary: Some("Retrieve station status".to_string()),
                    .. Default::default()
                }),
                put: Some(Operation {
                    summary: Some("Modify station status".to_string()),
                    .. Default::default()
                }),
                .. Default::default()
            },
            "/users".to_string(): PathItem {
                post: Some(Operation {
                    summary: Some("Register a user".to_string()),
                    .. Default::default()
                }),
                .. Default::default()
            },
            "/users/{name}".to_string(): PathItem {
                get: Some(Operation {
                    summary: Some("Retrieve user data".to_string()),
                    .. Default::default()
                }),
                put: Some(Operation {
                    summary: Some("Modify user data".to_string()),
                    .. Default::default()
                }),
                .. Default::default()
            },
            "/users/{name}/stations".to_string(): PathItem {
                get: Some(Operation {
                    summary: Some("List a users' registered stations".to_string()),
                    .. Default::default()
                }),
                post: Some(Operation {
                    summary: Some("Register a station (user side)".to_string()),
                    .. Default::default()
                }),
                .. Default::default()
            },
        },
        .. Default::default()
    }))
}

#[post("/v1/stations", data = "<req>")]
fn stations_post(req: Json<StationsReq>, db: State<DbConn>) -> ApiResp<StationsResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let token = Uuid::new_v4().to_simple().encode_lower(&mut Uuid::encode_buffer()).to_string();
    let hash = BasicAuth::from_parts(&req.id.to_string(), &token).hash();
    add_station(req.id, &req.name, &hash, &mut db)?;
    Ok(Json(StationsResp {
        token
    }))
}

#[get("/v1/stations/<id>")]
fn station_get(id: usize, db: State<DbConn>, auth: BasicAuth) -> ApiResp<StationResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
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
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
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
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let station = get_station(id, &mut db)?;
    if station.owner.as_ref().map(|o| Ok(auth.verify(&get_user(o, &mut db)?.pass))).unwrap_or(Ok(false))? {
        let data = get_data(station.id, &mut db)?;
        Ok(Json(DataResp {
            data: data.into_iter().map(|d| DataElement {
                time: d.time,
                val: d.val
            }).collect()
        }))
    } else {
        Err(Status::Unauthorized)
    }
}

#[post("/v1/stations/<id>/data", data = "<req>")]
fn data_post(id: usize, req: Json<DataReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let station = get_station(id, &mut db)?;
    if auth.verify(&station.token) {
        add_data(station.id, req.val, &mut db)?;
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/v1/stations/<id>/state")]
fn state_get(id: usize, db: State<DbConn>, auth: BasicAuth) -> ApiResp<StateResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
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
fn state_put(id: usize, req: Json<StateReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let mut station = get_station(id, &mut db)?;
    if auth.verify(&station.token) || station.owner.as_ref().map(|o| Ok(auth.verify(&get_user(o, &mut db)?.pass))).unwrap_or(Ok(false))? {
        station.state = req.state.clone();
        update_station(station, &mut db)?;
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[post("/v1/users", data = "<req>")]
fn users_post(req: Json<UsersReq>, db: State<DbConn>) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let hash = BasicAuth::from_parts(&req.name, &req.pass).hash();
    add_user(&req.name, &hash, &mut db)?;
    Ok(Json(EmptyResp {}))
}

#[get("/v1/users/<name>")]
fn user_get(name: String, db: State<DbConn>, auth: BasicAuth) -> ApiResp<UserResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let user = get_user(&name, &mut db)?;
    if auth.verify(&user.pass) {
        Ok(Json(UserResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[put("/v1/users/<name>", data = "<req>")]
fn user_put(name: String, req: Json<UserReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let mut user = get_user(&name, &mut db)?;
    if auth.verify(&user.pass) {
        let pass_hash = BasicAuth::from_parts(&user.name, &req.pass).hash();
        user.pass = pass_hash;
        update_user(user, &mut db)?;
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/v1/users/<name>/stations")]
fn user_stations_get(name: String, db: State<DbConn>, auth: BasicAuth) -> ApiResp<UserStationsResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let user = get_user(&name, &mut db)?;
    if auth.verify(&user.pass) {
        let stations = get_stations(&name, &mut db)?;
        Ok(Json(UserStationsResp {
            stations: stations.into_iter().map(|s| s.id).collect()
        }))
    } else {
        Err(Status::Unauthorized)
    }
}

#[post("/v1/users/<name>/stations", data = "<req>")]
fn user_stations_post(name: String, req: Json<UserStationsReq>, db: State<DbConn>, auth: BasicAuth) -> ApiResp<EmptyResp> {
    let mut db = db.lock().or(Err(Status::InternalServerError))?;
    let user = get_user(&name, &mut db)?;
    if auth.verify(&user.pass) {
        let mut station = get_station(req.id, &mut db)?;
        if station.owner.is_none() {
            station.owner = Some(name.clone());
            update_station(station, &mut db)?;
        }
        Ok(Json(EmptyResp {}))
    } else {
        Err(Status::Unauthorized)
    }
}

#[catch(401)] 
fn unauthorised(req: &Request) {
    println!("{:?}", req);
}

#[catch(404)] 
fn not_found(req: &Request) {
    println!("{:?}", req);
}

#[catch(500)] 
fn server_error(req: &Request) {
    println!("{:?}", req);
}

pub fn run(db_conn: DbConn) {
    rocket::ignite()
        .mount("/", routes![index, root, stations_post, station_get, station_put, data_get, data_post, state_get, state_put, users_post, user_get, user_put, user_stations_get, user_stations_post])
        .register(catchers![unauthorised, not_found, server_error])
        .manage(db_conn).launch();
}
