mod opencv;
mod service;

use futures::stream::TryStreamExt;
use hyper::http::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use multipart_async::server::Multipart;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;

#[macro_use]
extern crate log;

async fn svc(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri().clone();
    let resp = route(req).await.unwrap_or_else(|err| {
        err.log();
        ErrorResponseBody::from_error(err).into_response()
    });

    log_response(uri, &resp);
    Ok(resp)
}

async fn route(req: Request<Body>) -> Result<Response<Body>, Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/images") => store_img(req).await,
        _ => Err(Error::not_found("unknown route".to_string())),
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ImageData {
    URI(String),
    Base64(String),
    Bytes(Vec<u8>),
}

#[derive(Deserialize, Serialize)]
pub(crate) struct ImageRequest {
    pub(crate) filename: String,
    pub(crate) data: ImageData,
}

impl ImageRequest {
    async fn into_image(self) -> Result<service::Image, Error> {
        let img = match self.data {
            ImageData::URI(u) => service::Image::from_remote_source(self.filename, u).await?,
            ImageData::Base64(s) => service::Image::from_base64(self.filename, s)?,
            ImageData::Bytes(b) => service::Image::new(self.filename, b),
        };

        Ok(img)
    }
}

#[derive(Deserialize, Serialize)]
struct StoreImgRequestBody(Vec<ImageRequest>);

impl StoreImgRequestBody {
    async fn from_json_request(req: Request<Body>) -> Result<Self, Error> {
        let body = req.into_body().try_concat().await.or_internal_err()?;
        let req = serde_json::from_slice(&body.to_vec()).or_bad_request("invalid json")?;
        Ok(req)
    }

    async fn from_multipart_request(req: Request<Body>) -> Result<Self, Error> {
        let mut multipart = Multipart::try_from_request(req)
            .map_err(|_| Error::bad_request("invalid multipart form data".to_string()))?;

        let mut imgs = Vec::new();
        while let Some(field) = multipart.next_field().await.or_internal_err()? {
            let data = field.data.try_concat().await.or_internal_err()?;
            imgs.push(ImageRequest {
                filename: field.headers.name,
                data: ImageData::Bytes(data.to_vec()),
            })
        }

        Ok(Self(imgs))
    }
}

#[derive(Deserialize, Serialize)]
struct ImageResponse {
    filename: String,
}

impl ImageResponse {
    fn new(filename: String) -> Self {
        ImageResponse { filename: filename }
    }
}

#[derive(Deserialize, Serialize)]
struct StoreImgResponseBody(Vec<ImageResponse>);

impl StoreImgResponseBody {
    fn into_response(self) -> Result<Response<Body>, Error> {
        let json = serde_json::to_string(&self).or_internal_err()?;
        Response::builder()
            .status(StatusCode::CREATED)
            .body(Body::from(json))
            .or_internal_err()
    }
}

async fn store_img(req: Request<Body>) -> Result<Response<Body>, Error> {
    let headers = req.headers().clone();
    let req_body = match get_content_type(&headers).split(";").next() {
        Some("application/json") => StoreImgRequestBody::from_json_request(req).await,
        Some("multipart/form-data") => StoreImgRequestBody::from_multipart_request(req).await,
        _ => Err(Error::unsupported_media_type()),
    }
    .context("parse request body")?;

    let mut res = Vec::new();
    for img_req in req_body.0 {
        let img = img_req.into_image().await.context("load image")?;
        img.save().await.context("save image")?;
        res.push(ImageResponse::new(img.filename));
    }

    Ok(StoreImgResponseBody(res)
        .into_response()
        .context("build response")?)
}

fn get_content_type(headers: &HeaderMap<HeaderValue>) -> &str {
    headers
        .get(CONTENT_TYPE)
        .map(|v| v.to_str().unwrap_or_default())
        .unwrap_or_default()
}

fn log_response(uri: hyper::http::Uri, resp: &Response<Body>) {
    let status = resp.status().as_u16();
    let s = format!("status: {} route: {}", status, uri);
    match status {
        100..=299 => info!("{}", s),
        300..=499 => warn!("{}", s),
        _ => error!("{}", s),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let addr = ([127, 0, 0, 1], 3000).into();

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(svc)) });

    let server = Server::bind(&addr).serve(service);

    println!("Listening on http://{}", addr);
    server.await?;

    Ok(())
}

#[derive(Serialize)]
struct ErrorResponseBody {
    code: u16,
    reason: String,
}

impl ErrorResponseBody {
    fn from_error(mut err: Error) -> Self {
        if err.code.is_server_error() {
            err.cause = err.code.canonical_reason().unwrap_or_default().to_string()
        }

        ErrorResponseBody {
            code: err.code.as_u16(),
            reason: err.cause,
        }
    }

    fn into_response(self) -> Response<Body> {
        let json = serde_json::to_string(&self).expect("serialize json");
        Response::builder()
            .status(self.code)
            .body(Body::from(json))
            .expect("build response")
    }
}

#[derive(Debug)]
struct Error {
    code: StatusCode,
    backtrace: String,
    cause: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.backtrace, self.cause)
    }
}

impl Error {
    fn new(code: StatusCode, cause: String) -> Self {
        Self {
            code: code,
            backtrace: String::default(),
            cause: cause,
        }
    }

    fn bad_request(cause: String) -> Self {
        Self::new(StatusCode::BAD_REQUEST, cause)
    }

    fn internal(cause: String) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, cause)
    }

    fn not_found(cause: String) -> Self {
        Self::new(StatusCode::NOT_FOUND, cause)
    }

    fn unsupported_media_type() -> Self {
        let code = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        Self::new(code, Error::default_cause(code))
    }

    fn default_cause(code: StatusCode) -> String {
        code.canonical_reason().unwrap_or_default().to_string()
    }

    fn context(mut self, ctx: &str) -> Self {
        self.backtrace = format!("{}: {}", ctx, self.backtrace);
        self
    }

    fn log(&self) {
        if self.code == StatusCode::INTERNAL_SERVER_ERROR {
            error!("{}", self)
        }
    }
}

impl From<service::Error> for Error {
    fn from(err: service::Error) -> Self {
        match err.kind {
            service::ErrorKind::InvalidArgument(err) => {
                Self::bad_request(format!("{}: {}", err.arg_name, err.details))
            }
            service::ErrorKind::Internal => Self::internal(format!("{}", err.cause)),
        }
        .context(&err.backtrace)
    }
}

trait WrapError<T> {
    fn or_internal_err(self) -> Result<T, Error>;
    fn or_bad_request(self, details: &str) -> Result<T, Error>;
}

impl<T, E: StdError> WrapError<T> for Result<T, E> {
    fn or_internal_err(self) -> Result<T, Error> {
        self.map_err(|e| Error::internal(format!("{}", e)))
    }

    fn or_bad_request(self, details: &str) -> Result<T, Error> {
        self.map_err(|e| Error::bad_request(format!("{}: {}", details, e)))
    }
}

trait ErrorContext<T> {
    fn context(self, ctx: &str) -> Result<T, Error>;
}

impl<T> ErrorContext<T> for Result<T, Error> {
    fn context(self, ctx: &str) -> Result<T, Error> {
        self.map_err(|err| err.context(ctx))
    }
}

impl<T> ErrorContext<T> for Result<T, service::Error> {
    fn context(self, ctx: &str) -> Result<T, Error> {
        self.map_err(|err| Error::from(err).context(ctx))
    }
}
