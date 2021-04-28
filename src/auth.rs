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

use std::num::NonZeroU32;

use base64;
use ring::{digest, pbkdf2};
use rocket::Outcome;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};

#[derive(Debug)]
pub struct BasicAuth {
    pub user: String,
    pub pass: String
}

impl BasicAuth {
    pub fn from_header(header: &str) -> Option<Self> {
        if header.len() < 7 || &header[..6] != "Basic " {
            return None;
        }

        if let Ok(Ok(creds)) = base64::decode(&header[6..]).map(String::from_utf8) {
            if let Some((user, pass)) = creds.split_once(':') {
                return Some(Self { user: user.to_string(), pass: pass.to_string() })
            }
        }
        None
    }

    pub fn from_parts(user: &str, pass: &str) -> Self {
        Self { user: user.to_string(), pass: pass.to_string() }
    }

    pub fn hash(&self) -> String {
        let mut hash = [0u8; digest::SHA256_OUTPUT_LEN];
        pbkdf2::derive(pbkdf2::PBKDF2_HMAC_SHA256, NonZeroU32::new(100000).unwrap(), self.user.as_bytes(), self.pass.as_bytes(), &mut hash);
        base64::encode(hash)
    }

    pub fn verify(&self, hash: &str) -> bool {
        if let Ok(hash) = base64::decode(hash) {
            pbkdf2::verify(pbkdf2::PBKDF2_HMAC_SHA256, NonZeroU32::new(100000).unwrap(), self.user.as_bytes(), self.pass.as_bytes(), &hash).is_ok()
        } else {
            false
        }
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
