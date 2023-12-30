use crate::value::{to_value, Value};
use crate::ErrorKind;
use hyper::{
    body::{Body, Frame, SizeHint},
    http, HeaderMap, Response, StatusCode,
};
use hyper_static::{streamer::Empty, ErrorBoxed, Streamed};
use serde::Serialize;
use std::collections::VecDeque;
use std::error::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Full {
    data: Option<VecDeque<u8>>,
}

impl Full {
    pub fn new<D: Into<VecDeque<u8>>>(data: D) -> Self {
        Self {
            data: Some(data.into()),
        }
    }
}

impl Body for Full {
    type Data = VecDeque<u8>;
    type Error = ErrorBoxed;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(self.data.take().map(|d| Ok(Frame::data(d))))
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        self.data.as_ref().map_or_else(
            || SizeHint::with_exact(0),
            |data| SizeHint::with_exact(u64::try_from(data.len()).unwrap()),
        )
    }
}

pub const DEFAULT_MIME: &str = "application/octet-stream";

pub type BodyFull =
    Pin<Box<dyn Body<Error = Box<dyn Error + Send + Sync>, Data = VecDeque<u8>> + Send>>;

#[macro_export]
macro_rules! hyper_response {
    ($code: expr, $message: expr) => {
        hyper::Response::builder()
            .status($code)
            .body($message.into_body())
    };
    ($code: expr) => {
        hyper::Response::builder().status($code).body(Empty::new())
    };
}

pub type HResult = std::result::Result<HContent, crate::Error>;

pub trait HResultX {
    fn into_hyper_response(self) -> Result<Streamed, http::Error>;
}

#[inline]
fn into_body<D>(data: D) -> BodyFull
where
    D: Into<VecDeque<u8>>,
{
    Box::pin(Full::new(Into::<VecDeque<u8>>::into(data)))
}

#[inline]
pub fn body_empty() -> BodyFull {
    Box::pin(Empty::new())
}

pub trait IntoBodyFull {
    fn into_body(self) -> BodyFull;
}

impl IntoBodyFull for Vec<u8> {
    #[inline]
    fn into_body(self) -> BodyFull {
        into_body(self)
    }
}

impl IntoBodyFull for String {
    #[inline]
    fn into_body(self) -> BodyFull {
        into_body(self.as_bytes().to_vec())
    }
}

impl<'a> IntoBodyFull for &'a str {
    #[inline]
    fn into_body(self) -> BodyFull {
        into_body(self.as_bytes().to_vec())
    }
}

impl HResultX for HResult {
    fn into_hyper_response(self) -> Result<Streamed, http::Error> {
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
                    let mut result = r.status(StatusCode::OK).body(v.into_body())?;
                    if let Some(h) = header_map {
                        result.headers_mut().extend(h);
                    }
                    Ok(result)
                }
                HContent::Value(val) => match serde_json::to_vec(&val) {
                    Ok(v) => Response::builder()
                        .header(hyper::header::CONTENT_TYPE, "application/json")
                        .status(StatusCode::OK)
                        .body(v.into_body()),
                    Err(e) => {
                        hyper_response!(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    }
                },
                HContent::Redirect(l) => Response::builder()
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header(hyper::header::LOCATION, l)
                    .body(body_empty()),
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
    HyperResult(Result<Streamed, http::Error>),
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
