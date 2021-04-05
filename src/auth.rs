use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::Outcome;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;

use base64;

#[derive(Debug)]
pub struct BasicAuth {
    pub creds: String,
}

impl BasicAuth {
    pub fn from_header(header: &str) -> Option<Self> {
        if header.len() < 7 || &header[..6] != "Basic " {
            return None;
        }

        let creds = match base64::decode(&header[6..]) {
            Ok(creds) => String::from_utf8(creds).unwrap(),
            Err(_) => return None,
        };

        Some(Self { creds })
    }

    pub fn from_parts(user: &str, pass: &str) -> Self {
        Self { creds: format!("{}:{}", user, pass) }
    }

    pub fn hash(&self) -> String {
        Argon2::default().hash_password_simple(self.creds.as_bytes(), SaltString::generate(&mut OsRng).as_ref())
            .unwrap().to_string()
    }

    pub fn verify(&self, hash: &str) -> bool {
        Argon2::default().verify_password(self.creds.as_bytes(), &PasswordHash::new(hash).unwrap()).is_ok()
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for BasicAuth {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let keys: Vec<&str> = request.headers().get("Authorization").collect();
        if keys.len() == 1 {
            match BasicAuth::from_header(keys[0]) {
                Some(auth) => Outcome::Success(auth),
                None => Outcome::Failure((Status::BadRequest, ()))
            }
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}
