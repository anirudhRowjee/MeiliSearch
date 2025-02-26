use std::error::Error;
use std::fmt;

use actix_web as aweb;
use actix_web::body::Body;
use actix_web::http::StatusCode;
use actix_web::HttpResponseBuilder;
use aweb::error::{JsonPayloadError, QueryPayloadError};
use meilisearch_error::{Code, ErrorCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum MeilisearchHttpError {
    #[error("A Content-Type header is missing. Accepted values for the Content-Type header are: \"application/json\", \"application/x-ndjson\", \"text/csv\"")]
    MissingContentType,
    #[error("The Content-Type \"{0}\" is invalid. Accepted values for the Content-Type header are: \"application/json\", \"application/x-ndjson\", \"text/csv\"")]
    InvalidContentType(String),
}

impl ErrorCode for MeilisearchHttpError {
    fn error_code(&self) -> Code {
        match self {
            MeilisearchHttpError::MissingContentType => Code::MissingContentType,
            MeilisearchHttpError::InvalidContentType(_) => Code::InvalidContentType,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResponseError {
    #[serde(skip)]
    code: StatusCode,
    message: String,
    error_code: String,
    error_type: String,
    error_link: String,
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl<T> From<T> for ResponseError
where
    T: ErrorCode,
{
    fn from(other: T) -> Self {
        Self {
            code: other.http_status(),
            message: other.to_string(),
            error_code: other.error_name(),
            error_type: other.error_type(),
            error_link: other.error_url(),
        }
    }
}

impl aweb::error::ResponseError for ResponseError {
    fn error_response(&self) -> aweb::HttpResponse<Body> {
        let json = serde_json::to_vec(self).unwrap();
        HttpResponseBuilder::new(self.status_code())
            .content_type("application/json")
            .body(json)
    }

    fn status_code(&self) -> StatusCode {
        self.code
    }
}

impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadError::Json(e) => e.fmt(f),
            PayloadError::Query(e) => e.fmt(f),
        }
    }
}

#[derive(Debug)]
pub enum PayloadError {
    Json(JsonPayloadError),
    Query(QueryPayloadError),
}

impl Error for PayloadError {}

impl ErrorCode for PayloadError {
    fn error_code(&self) -> Code {
        match self {
            PayloadError::Json(err) => match err {
                JsonPayloadError::Overflow { .. } => Code::PayloadTooLarge,
                JsonPayloadError::ContentType => Code::UnsupportedMediaType,
                JsonPayloadError::Payload(aweb::error::PayloadError::Overflow) => {
                    Code::PayloadTooLarge
                }
                JsonPayloadError::Deserialize(_) | JsonPayloadError::Payload(_) => Code::BadRequest,
                JsonPayloadError::Serialize(_) => Code::Internal,
                _ => Code::Internal,
            },
            PayloadError::Query(err) => match err {
                QueryPayloadError::Deserialize(_) => Code::BadRequest,
                _ => Code::Internal,
            },
        }
    }
}

impl From<JsonPayloadError> for PayloadError {
    fn from(other: JsonPayloadError) -> Self {
        Self::Json(other)
    }
}

impl From<QueryPayloadError> for PayloadError {
    fn from(other: QueryPayloadError) -> Self {
        Self::Query(other)
    }
}

pub fn payload_error_handler<E>(err: E) -> ResponseError
where
    E: Into<PayloadError>,
{
    err.into().into()
}
