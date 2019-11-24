use super::libvips;
use std::io::ErrorKind::NotFound as IONotFound;
use std::path::Path;
use tokio::{fs::File, prelude::*};

pub(crate) struct Image {
    pub(crate) filename: String,
    pub(crate) data: Vec<u8>,
}

impl Image {
    pub(crate) fn new(filename: String, data: Vec<u8>) -> Self {
        Image {
            filename: filename,
            data: data,
        }
    }

    pub(crate) async fn from_remote_source(filename: String, uri: String) -> Result<Self, Error> {
        let client = reqwest::Client::new();
        let res = client
            .get(&uri)
            .send()
            .await
            .or_invalid_argument("uri", "failed to fetch specified file")?;

        let body = res
            .bytes()
            .await
            .or_internal_err()
            .context("get response bytes")?;

        Ok(Self::new(filename, body.to_vec()))
    }

    pub(crate) async fn from_base64(filename: String, data: String) -> Result<Self, Error> {
        let bytes = tokio_executor::blocking::run(move || base64::decode(&data))
            .await
            .or_invalid_argument("base64", "failed to decode")?;

        Ok(Self::new(filename, bytes))
    }

    pub(crate) async fn from_storage(filename: String, folder: &Path) -> Result<Self, Error> {
        let mut data = Vec::new();

        let mut file = File::open(folder.join(&filename))
            .await
            .map_err(|e| Error::map_io(e, IONotFound, ErrorKind::NotFound))
            .context("open file")?;

        file.read_to_end(&mut data)
            .await
            .or_internal_err()
            .context("read file")?;

        Ok(Image::new(filename, data))
    }

    pub(crate) async fn into_thumbnail(self) -> Result<Self, Error> {
        let data = self.data;
        let res = tokio_executor::blocking::run(|| libvips::thumbnail(data))
            .await
            .or_internal_err()?;

        Ok(Image::new(self.filename, res))
    }

    pub(crate) async fn save(&self, path: &Path) -> Result<(), Error> {
        let mut file = File::create(path.join(&self.filename))
            .await
            .or_internal_err()
            .context("create file")?;

        file.write_all(&self.data)
            .await
            .or_internal_err()
            .context("write file")?;

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Error {
    pub(crate) kind: ErrorKind,
    pub(crate) cause: ErrorCause,
    pub(crate) backtrace: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.backtrace, self.cause)
    }
}

impl Error {
    fn new<T: Into<ErrorCause>>(kind: ErrorKind, cause: T) -> Self {
        Error {
            kind: kind,
            backtrace: String::default(),
            cause: cause.into(),
        }
    }

    fn invalid_argument<T: Into<ErrorCause>>(arg: &str, details: &str, cause: T) -> Self {
        Self::new(
            ErrorKind::InvalidArgument(InvalidArgumentError::new(arg, details)),
            cause,
        )
    }

    fn internal<T: Into<ErrorCause>>(cause: T) -> Self {
        Self::new(ErrorKind::Internal, cause)
    }

    fn context(mut self, ctx: &str) -> Self {
        self.backtrace = format!("{}: {}", ctx, self.backtrace);
        self
    }

    fn map_io(err: std::io::Error, io_kind: std::io::ErrorKind, kind: ErrorKind) -> Self {
        if err.kind() == io_kind {
            return Error::new(kind, err);
        };

        Error::new(ErrorKind::Internal, err)
    }
}

#[derive(Debug)]
pub(crate) struct InvalidArgumentError {
    pub(crate) arg_name: String,
    pub(crate) details: String,
}

impl InvalidArgumentError {
    fn new(arg: &str, details: &str) -> Self {
        Self {
            arg_name: arg.to_string(),
            details: details.to_string(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum ErrorCause {
    IO(std::io::Error),
    Reqwest(reqwest::Error),
    Base64Decode(base64::DecodeError),
    Libvips(libvips::Error),
}

impl std::fmt::Display for ErrorCause {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorCause::IO(err) => write!(f, "io: {}", err),
            ErrorCause::Reqwest(err) => write!(f, "reqwest: {}", err),
            ErrorCause::Base64Decode(err) => write!(f, "base64: {}", err),
            ErrorCause::Libvips(err) => write!(f, "libvips: {}", err),
        }
    }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    InvalidArgument(InvalidArgumentError),
    NotFound,
    Internal,
}

impl From<std::io::Error> for ErrorCause {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<reqwest::Error> for ErrorCause {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err)
    }
}

impl From<base64::DecodeError> for ErrorCause {
    fn from(err: base64::DecodeError) -> Self {
        Self::Base64Decode(err)
    }
}

impl From<libvips::Error> for ErrorCause {
    fn from(err: libvips::Error) -> Self {
        Self::Libvips(err)
    }
}

trait WrapError<T> {
    fn or_invalid_argument(self, arg: &str, details: &str) -> Result<T, Error>;
    fn or_internal_err(self) -> Result<T, Error>;
}

impl<T, E: Into<ErrorCause>> WrapError<T> for Result<T, E> {
    fn or_invalid_argument(self, arg: &str, details: &str) -> Result<T, Error> {
        self.map_err(|e| Error::invalid_argument(arg, details, e.into()))
    }

    fn or_internal_err(self) -> Result<T, Error> {
        self.map_err(|e| Error::internal(e.into()))
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
