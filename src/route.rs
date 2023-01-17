use core::str;
use std::{collections::BTreeMap, net::IpAddr, str::FromStr};

use actix_web::{
  get,
  http::{Method, StatusCode},
  post,
  web::{self, Form, Json},
  Either, HttpResponse, Responder,
};
use clap::{crate_name, crate_version};
use indoc::indoc;
use serde::{Deserialize, Serialize};

use crate::{IPV4_DB, IPV6_DB, TOKEN};

#[get("/")]
pub async fn root() -> impl Responder {
  HttpResponse::Ok().body(format!(
    indoc! {"
        {name} v{version}

        Server launched.
    "},
    name = crate_name!(),
    version = crate_version!()
  ))
}

#[derive(Deserialize)]
pub struct IpReq {
  token: Option<String>,
  ip: String,
  language: String,
}

#[derive(Serialize)]
struct IpResp {
  ok: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  error: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  fields: Option<BTreeMap<String, String>>,
}

impl IpResp {
  fn error(message: &str) -> IpResp {
    IpResp {
      ok: false,
      error: Some(message.to_string()),
      fields: None,
    }
  }
  fn ok(fields: BTreeMap<String, String>) -> IpResp {
    IpResp {
      ok: true,
      error: None,
      fields: Some(fields),
    }
  }
}

#[post("ip")]
pub async fn ip(req: Either<Json<IpReq>, Form<IpReq>>) -> impl Responder {
  let IpReq {
    token,
    ip,
    language,
  } = req.into_inner();
  let not_trusted = if let Some(server_token) = TOKEN.get().unwrap() {
    if let Some(token) = token {
      server_token.as_str() != token.as_str()
    } else {
      true
    }
  } else {
    false
  };

  if not_trusted {
    return Either::Right(HttpResponse::Unauthorized().finish());
  };

  let addr = match IpAddr::from_str(ip.as_str()) {
    Ok(addr) => addr,
    Err(_) => {
      return Either::Left(web::Json(IpResp::error(
        format!("Failed to parse ip `{ip}` to IpAddr").as_str(),
      )))
    }
  };

  let result = match addr {
    IpAddr::V4(_) => {
      let Some(db) = IPV4_DB.get() else {
        return Either::Left(web::Json(IpResp::error("IPv4 IPDB is not supported")));
      };
      db.find_to_map(addr, &language)
    }
    IpAddr::V6(_) => {
      let Some(db) = IPV6_DB.get() else {
        return Either::Left(web::Json(IpResp::error("IPv6 IPDB is not supported")));
      };
      db.find_to_map(addr, &language)
    }
  };

  let resp = match result {
    Ok(fields) => IpResp::ok(fields),
    Err(err) => IpResp::error(format!("Failed to find IP info: {err}").as_str()),
  };

  Either::Left(web::Json(resp))
}

pub async fn default(req_method: Method) -> impl Responder {
  match req_method {
    Method::GET => {
      let text = "Not Found".customize().with_status(StatusCode::NOT_FOUND);
      Either::Left(text)
    }
    _ => Either::Right(HttpResponse::MethodNotAllowed().finish()),
  }
}
