use crate::value::{to_value, Value};
use crate::ErrorKind;
use hyper::{http, Body, HeaderMap, Response, StatusCode};
use serde::Serialize;
use std::error::Error;

pub const DEFAULT_MIME: &str = "application/octet-stream";

#[macro_export]
macro_rules! hyper_response {
    ($code: expr, $message: expr) => {
        hyper::Response::builder()
            .status($code)
            .body(Body::from($message))
    };
    ($code: expr) => {
        hyper::Response::builder().status($code).body(Body::empty())
    };
}

pub type HResult = std::result::Result<HContent, crate::Error>;

pub trait HResultX {
    fn into_hyper_response(self) -> Result<Response<Body>, http::Error>;
}

impl HResultX for HResult {
    fn into_hyper_response(self) -> Result<Response<Body>, http::Error> {
        match self {
            Ok(resp) => match resp {
                HContent::Data(v, mime, header_map) => {
                    let mut r = Response::builder();
                    if let Some(mt) = mime {
                        if mt.starts_with("text/") {
                            r = r.header(
                                hyper::header::CONTENT_TYPE,
                                &format!("{};charset=utf-8", mt),
                            );
                        } else {
                            r = r.header(hyper::header::CONTENT_TYPE, mt);
                        }
                    } else {
                        r = r.header(hyper::header::CONTENT_TYPE, DEFAULT_MIME);
                    }
                    let mut result = r.status(StatusCode::OK).body(Body::from(v))?;
                    if let Some(h) = header_map {
                        result.headers_mut().extend(h);
                    }
                    Ok(result)
                }
                HContent::Value(val) => match serde_json::to_vec(&val) {
                    Ok(v) => Response::builder()
                        .header(hyper::header::CONTENT_TYPE, "application/json")
                        .status(StatusCode::OK)
                        .body(Body::from(v)),
                    Err(e) => {
                        hyper_response!(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    }
                },
                HContent::Redirect(l) => Response::builder()
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header(hyper::header::LOCATION, l)
                    .body(Body::empty()),
                HContent::HyperResult(r) => r,
            },
            Err(e) if e.kind() == ErrorKind::ResourceNotFound => {
                hyper_response!(StatusCode::NOT_FOUND, e.to_string())
            }
            Err(e) if e.kind() == ErrorKind::AccessDenied => {
                hyper_response!(StatusCode::FORBIDDEN, e.to_string())
            }
            Err(e) if e.kind() == ErrorKind::InvalidParameter => {
                hyper_response!(StatusCode::BAD_REQUEST, e.to_string())
            }
            Err(e) => {
                hyper_response!(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        }
    }
}

pub enum HContent {
    Data(Vec<u8>, Option<&'static str>, Option<HeaderMap>),
    Value(Value),
    Redirect(String),
    HyperResult(Result<Response<Body>, http::Error>),
}

impl HContent {
    /// # Panics
    ///
    /// Should not panic
    pub fn ok() -> Self {
        #[derive(Serialize)]
        struct OK {
            ok: bool,
        }
        HContent::Value(to_value(OK { ok: true }).unwrap())
    }
    /// # Panics
    ///
    /// Should not panic
    pub fn not_ok() -> Self {
        #[derive(Serialize)]
        struct OK {
            ok: bool,
        }
        HContent::Value(to_value(OK { ok: false }).unwrap())
    }
}

impl From<hyper_static::serve::Error> for crate::Error {
    fn from(e: hyper_static::serve::Error) -> Self {
        match e.kind() {
            hyper_static::serve::ErrorKind::Internal => {
                Self::newc(ErrorKind::FunctionFailed, e.source())
            }
            hyper_static::serve::ErrorKind::NotFound => {
                Self::newc(ErrorKind::ResourceNotFound, e.source())
            }
            hyper_static::serve::ErrorKind::Forbidden => {
                Self::newc(ErrorKind::AccessDenied, e.source())
            }
            hyper_static::serve::ErrorKind::BadRequest => {
                Self::newc(ErrorKind::InvalidParameter, e.source())
            }
        }
    }
}
