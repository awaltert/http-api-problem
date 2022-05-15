//! # HTTP-API-PROBLEM
//!
//! [![crates.io](https://img.shields.io/crates/v/http-api-problem.svg)](https://crates.io/crates/http-api-problem)
//! [![docs.rs](https://docs.rs/http-api-problem/badge.svg)](https://docs.rs/http-api-problem)
//! [![downloads](https://img.shields.io/crates/d/http-api-problem.svg)](https://crates.io/crates/http-api-problem)
//! ![CI](https://github.com/chridou/http-api-problem/workflows/CI/badge.svg)
//! [![license-mit](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/chridou/http-api-problem/blob/master/LICENSE-MIT)
//! [![license-apache](http://img.shields.io/badge/license-APACHE-blue.svg)](https://github.com/chridou/http-api-problem/blob/master/LICENSE-APACHE)
//!
//! A library to create HTTP response content for APIs based on
//! [RFC7807](https://tools.ietf.org/html/rfc7807).
//!
//! ## Usage
//!
//! Get the latest version for your `Cargo.toml` from
//! [crates.io](https://crates.io/crates/http-api-problem).
//!
//! Add this to your crate root:
//!
//! ```rust
//! use http_api_problem;
//! ```
//!
//!  ## serde
//!
//! [HttpApiProblem] implements [Serialize] and [Deserialize] for
//! [HttpApiProblem].
//!
//! ## Examples
//!
//! ```rust
//! use http_api_problem::*;
//!
//! let p = HttpApiProblem::new(StatusCode::UNPROCESSABLE_ENTITY)
//!     .title("You do not have enough credit.")
//!     .detail("Your current balance is 30, but that costs 50.")
//!     .type_url("https://example.com/probs/out-of-credit")
//!     .instance("/account/12345/msgs/abc");
//!
//! assert_eq!(Some(StatusCode::UNPROCESSABLE_ENTITY), p.status);
//! assert_eq!(Some("You do not have enough credit."), p.title.as_deref());
//! assert_eq!(Some("Your current balance is 30, but that costs 50."), p.detail.as_deref());
//! assert_eq!(Some("https://example.com/probs/out-of-credit"), p.type_url.as_deref());
//! assert_eq!(Some("/account/12345/msgs/abc"), p.instance.as_deref());
//! ```
//!
//! There is also `TryFrom<u16>` implemented for [StatusCode]:
//!
//! ```rust
//! use http_api_problem::*;
//!
//! let p = HttpApiProblem::try_new(422).unwrap()
//!     .title("You do not have enough credit.")
//!     .detail("Your current balance is 30, but that costs 50.")
//!     .type_url("https://example.com/probs/out-of-credit")
//!     .instance("/account/12345/msgs/abc");
//!
//! assert_eq!(Some(StatusCode::UNPROCESSABLE_ENTITY), p.status);
//! assert_eq!(Some("You do not have enough credit."), p.title.as_deref());
//! assert_eq!(Some("Your current balance is 30, but that costs 50."), p.detail.as_deref());
//! assert_eq!(Some("https://example.com/probs/out-of-credit"), p.type_url.as_deref());
//! assert_eq!(Some("/account/12345/msgs/abc"), p.instance.as_deref());
//! ```
//!
//! ## Status Codes
//!
//! The specification does not require the [HttpApiProblem] to contain a
//! status code. Nevertheless this crate supports creating responses
//! for web frameworks. Responses require a status code. If no status code
//! was set on the [HttpApiProblem] `500 - Internal Server Error` will be
//! used as a fallback. This can be easily avoided by only using those constructor
//! functions which require a [StatusCode].
//!
//! ## Features
//!
//! ### Web Frameworks
//!
//! There are multiple features to integrate with web frameworks:
//!
//! * `warp`
//! * `hyper`
//! * `actix-web`
//! * `salvo`
//! * `tide`
//! * `rocket (v0.5.0-rc1)`
//!
//! These mainly convert the `HttpApiProblem` to response types of
//! the frameworks and implement traits to integrate with the frameworks
//! error handling
//!
//! ### ApiError
//!
//! The feature `api-error` enables a structure which can be
//! return from "api handlers" that generate responses and can be
//! converted into an `HttpApiProblem`.
//!
//! ## License
//!
//! `http-api-problem` is primarily distributed under the terms of both the MIT
//! license and the Apache License (Version 2.0).
//!
//! Copyright (c) 2017 Christian Douven.
use std::convert::TryInto;
use std::error::Error;
use std::fmt;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "api-error")]
mod api_error;
#[cfg(feature = "api-error")]
pub use api_error::*;

#[cfg(feature = "hyper")]
use hyper;

#[cfg(feature = "actix-web")]
use actix_web_crate as actix_web;

#[cfg(feature = "salvo")]
use salvo;

pub use http::status::{InvalidStatusCode, StatusCode};

/// The recommended media type when serialized to JSON
///
/// "application/problem+json"
pub static PROBLEM_JSON_MEDIA_TYPE: &str = "application/problem+json";

/// Description of a problem that can be returned by an HTTP API
/// based on [RFC7807](https://tools.ietf.org/html/rfc7807)
///
/// # Example
///
/// ```javascript
/// {
///    "type": "https://example.com/probs/out-of-credit",
///    "title": "You do not have enough credit.",
///    "detail": "Your current balance is 30, but that costs 50.",
///    "instance": "/account/12345/msgs/abc",
/// }
/// ```
///
/// # Status Codes and Responses
///
/// Prefer to use one of the constructors which
/// ensure that a [StatusCode] is set. If no [StatusCode] is
/// set and a transformation to a response of a web framework
/// is made a [StatusCode] becomes mandatory which in this case will
/// default to `500`.
///
/// When receiving an [HttpApiProblem] there might be an invalid
/// [StatusCode] contained. In this case the `status` field will be empty.
/// This is a trade off so that the recipient does not have to deal with
/// another error and can still have access to the remaining fields of the
/// struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct HttpApiProblem {
    /// A URI reference [RFC3986](https://tools.ietf.org/html/rfc3986) that identifies the
    /// problem type.  This specification encourages that, when
    /// dereferenced, it provide human-readable documentation for the
    /// problem type (e.g., using HTML [W3C.REC-html5-20141028]).  When
    /// this member is not present, its value is assumed to be
    /// "about:blank".
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_url: Option<String>,
    /// The HTTP status code [RFC7231, Section 6](https://tools.ietf.org/html/rfc7231#section-6)
    /// generated by the origin server for this occurrence of the problem.
    #[serde(default)]
    #[serde(with = "custom_http_status_serialization")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusCode>,
    /// A short, human-readable summary of the problem
    /// type. It SHOULD NOT change from occurrence to occurrence of the
    /// problem, except for purposes of localization (e.g., using
    /// proactive content negotiation;
    /// see [RFC7231, Section 3.4](https://tools.ietf.org/html/rfc7231#section-3.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A human-readable explanation specific to this
    /// occurrence of the problem.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// A URI reference that identifies the specific
    /// occurrence of the problem.  It may or may not yield further
    /// information if dereferenced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    /// Additional fields that must be JSON values
    #[serde(flatten)]
    additional_fields: HashMap<String, serde_json::Value>,
}

impl HttpApiProblem {
    /// Creates a new instance with the given [StatusCode].
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::new(StatusCode::INTERNAL_SERVER_ERROR);
    ///
    /// assert_eq!(Some(StatusCode::INTERNAL_SERVER_ERROR), p.status);
    /// assert_eq!(None, p.title);
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn new<T: Into<StatusCode>>(status: T) -> Self {
        Self::empty().status(status)
    }

    /// Creates a new instance with the given [StatusCode].
    ///
    /// Fails if the argument can not be converted into a [StatusCode].
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::try_new(500).unwrap();
    ///
    /// assert_eq!(Some(StatusCode::INTERNAL_SERVER_ERROR), p.status);
    /// assert_eq!(None, p.title);
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn try_new<T: TryInto<StatusCode>>(status: T) -> Result<Self, InvalidStatusCode>
    where
        T::Error: Into<InvalidStatusCode>,
    {
        let status = status.try_into().map_err(|e| e.into())?;
        Ok(Self::new(status))
    }

    /// Creates a new instance with `title` derived from a [StatusCode].
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::with_title(StatusCode::NOT_FOUND);
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(Some("Not Found"), p.title.as_deref());
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn with_title<T: Into<StatusCode>>(status: T) -> Self {
        let status = status.into();
        Self::new(status).title(
            status
                .canonical_reason()
                .unwrap_or("<unknown status code>")
                .to_string(),
        )
    }

    /// Creates a new instance with `title` derived from a [StatusCode].
    ///
    /// Fails if the argument can not be converted into a [StatusCode].
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::try_with_title(404).unwrap();
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(Some("Not Found"), p.title.as_deref());
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn try_with_title<T: TryInto<StatusCode>>(status: T) -> Result<Self, InvalidStatusCode>
    where
        T::Error: Into<InvalidStatusCode>,
    {
        let status = status.try_into().map_err(|e| e.into())?;
        Ok(Self::with_title(status))
    }

    /// Creates a new instance with the `title` and `type_url` derived from the
    /// [StatusCode].
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::with_title_and_type(StatusCode::SERVICE_UNAVAILABLE);
    ///
    /// assert_eq!(Some(StatusCode::SERVICE_UNAVAILABLE), p.status);
    /// assert_eq!(Some("Service Unavailable"), p.title.as_deref());
    /// assert_eq!(None, p.detail);
    /// assert_eq!(Some("https://httpstatuses.com/503".to_string()), p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn with_title_and_type<T: Into<StatusCode>>(status: T) -> Self {
        let status = status.into();
        Self::with_title(status).type_url(format!("https://httpstatuses.com/{}", status.as_u16()))
    }

    /// Creates a new instance with the `title` and `type_url` derived from the
    /// [StatusCode].
    ///
    /// Fails if the argument can not be converted into a [StatusCode].
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::try_with_title_and_type(503).unwrap();
    ///
    /// assert_eq!(Some(StatusCode::SERVICE_UNAVAILABLE), p.status);
    /// assert_eq!(Some("Service Unavailable"), p.title.as_deref());
    /// assert_eq!(None, p.detail);
    /// assert_eq!(Some("https://httpstatuses.com/503".to_string()), p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn try_with_title_and_type<T: TryInto<StatusCode>>(
        status: T,
    ) -> Result<Self, InvalidStatusCode>
    where
        T::Error: Into<InvalidStatusCode>,
    {
        let status = status.try_into().map_err(|e| e.into())?;

        Ok(Self::with_title_and_type(status))
    }

    /// Creates a new instance without any field set.
    ///
    /// Prefer to use one of the other constructors which
    /// ensure that a [StatusCode] is set. If no [StatusCode] is
    /// set and a transformation to a response of a web framework
    /// is made a [StatusCode] becomes mandatory which in this case will
    /// default to `500`.
    pub fn empty() -> Self {
        HttpApiProblem {
            type_url: None,
            status: None,
            title: None,
            detail: None,
            instance: None,
            additional_fields: Default::default(),
        }
    }

    /// Sets the `status`
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::new(StatusCode::NOT_FOUND).title("Error");
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(Some("Error"), p.title.as_deref());
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn status<T: Into<StatusCode>>(mut self, status: T) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Sets the `type_url`
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::new(StatusCode::NOT_FOUND).type_url("http://example.com/my/real_error");
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(None, p.title);
    /// assert_eq!(None, p.detail);
    /// assert_eq!(Some("http://example.com/my/real_error".to_string()), p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn type_url<T: Into<String>>(mut self, type_url: T) -> Self {
        self.type_url = Some(type_url.into());
        self
    }

    /// Tries to set the `status`
    ///
    /// Fails if the argument can not be converted into a [StatusCode].
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::try_new(404).unwrap().title("Error");
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(Some("Error"), p.title.as_deref());
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn try_status<T: TryInto<StatusCode>>(
        mut self,
        status: T,
    ) -> Result<Self, InvalidStatusCode>
    where
        T::Error: Into<InvalidStatusCode>,
    {
        self.status = Some(status.try_into().map_err(|e| e.into())?);
        Ok(self)
    }

    /// Sets the `title`
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::new(StatusCode::NOT_FOUND).title("Another Error");
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(Some("Another Error"), p.title.as_deref());
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the `detail`
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::new(StatusCode::NOT_FOUND).detail("a detailed description");
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(None, p.title);
    /// assert_eq!(Some("a detailed description".to_string()), p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(None, p.instance);
    /// ```
    pub fn detail<T: Into<String>>(mut self, detail: T) -> HttpApiProblem {
        self.detail = Some(detail.into());
        self
    }

    /// Sets the `instance`
    ///
    /// #Example
    ///
    /// ```rust
    /// use http_api_problem::*;
    ///
    /// let p = HttpApiProblem::new(StatusCode::NOT_FOUND).instance("/account/1234/withdraw");
    ///
    /// assert_eq!(Some(StatusCode::NOT_FOUND), p.status);
    /// assert_eq!(None, p.title);
    /// assert_eq!(None, p.detail);
    /// assert_eq!(None, p.type_url);
    /// assert_eq!(Some("/account/1234/withdraw".to_string()), p.instance);
    /// ```
    pub fn instance<T: Into<String>>(mut self, instance: T) -> HttpApiProblem {
        self.instance = Some(instance.into());
        self
    }

    /// Add a value that must be serializable.
    ///
    /// The key must not be one of the field names of this struct.
    pub fn try_value<K, V>(mut self, key: K, value: &V) -> Result<Self, String>
    where
        V: Serialize,
        K: Into<String>,
    {
        self.try_set_value(key, value)?;
        Ok(self)
    }

    /// Add a value that must be serializable.
    ///
    /// The key must not be one of the field names of this struct.
    /// If the key is a field name or the value is not serializable nothing happens.
    pub fn value<K, V>(mut self, key: K, value: &V) -> Self
    where
        V: Serialize,
        K: Into<String>,
    {
        self.set_value(key, value);
        self
    }

    pub fn set_value<K, V>(&mut self, key: K, value: &V)
    where
        V: Serialize,
        K: Into<String>,
    {
        let _ = self.try_set_value(key, value);
    }

    /// Returns the deserialized field for the given key.
    ///
    /// If the key does not exist or the field is not deserializable to
    /// the target type `None` is returned
    pub fn get_value<K, V>(&self, key: &str) -> Option<V>
    where
        V: DeserializeOwned,
    {
        self.json_value(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn try_set_value<K, V>(&mut self, key: K, value: &V) -> Result<(), String>
    where
        V: Serialize,
        K: Into<String>,
    {
        let key: String = key.into();
        match key.as_ref() {
            "type" => return Err("'type' is a reserved field name".into()),
            "status" => return Err("'status' is a reserved field name".into()),
            "title" => return Err("'title' is a reserved field name".into()),
            "detail" => return Err("'detail' is a reserved field name".into()),
            "instance" => return Err("'instance' is a reserved field name".into()),
            "additional_fields" => {
                return Err("'additional_fields' is a reserved field name".into());
            }
            _ => (),
        }
        let serialized = serde_json::to_value(value).map_err(|err| err.to_string())?;
        self.additional_fields.insert(key, serialized);
        Ok(())
    }

    pub fn keys<K, V>(&self) -> impl Iterator<Item = &String>
    where
        V: DeserializeOwned,
    {
        self.additional_fields.keys()
    }

    /// Returns the `serde_json::Value` for the given key if the key exists.
    pub fn json_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.additional_fields.get(key)
    }

    /// Serialize to a JSON `Vec<u8>`
    pub fn json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    /// Serialize to a JSON `String`
    pub fn json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Creates a [hyper] response.
    ///
    /// If status is `None` `500 - Internal Server Error` is the
    /// default.
    ///
    /// Requires the `hyper` feature
    #[cfg(feature = "hyper")]
    pub fn to_hyper_response(&self) -> hyper::Response<hyper::Body> {
        use hyper::header::{HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};
        use hyper::*;

        let json = self.json_bytes();
        let length = json.len() as u64;

        let (mut parts, body) = Response::new(json.into()).into_parts();

        parts.headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(PROBLEM_JSON_MEDIA_TYPE),
        );
        parts.headers.insert(
            CONTENT_LENGTH,
            HeaderValue::from_str(&length.to_string()).unwrap(),
        );
        parts.status = self.status_or_internal_server_error();

        Response::from_parts(parts, body)
    }

    /// Creates an `actix` response.
    ///
    /// If status is `None` or not convertible
    /// to an actix status `500 - Internal Server Error` is the
    /// default.
    ///
    /// Requires the `actix-web` feature
    #[cfg(feature = "actix-web")]
    pub fn to_actix_response(&self) -> actix_web::HttpResponse {
        let effective_status = self.status_or_internal_server_error();
        let actix_status = actix_web::http::StatusCode::from_u16(effective_status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

        let json = self.json_bytes();

        actix_web::HttpResponse::build(actix_status)
            .append_header((
                actix_web::http::header::CONTENT_TYPE,
                PROBLEM_JSON_MEDIA_TYPE,
            ))
            .body(json)
    }

    /// Creates a `rocket` response.
    ///
    /// If status is `None` `500 - Internal Server Error` is the
    /// default.
    ///
    /// Requires the `rocket` feature
    #[cfg(feature = "rocket")]
    pub fn to_rocket_response(&self) -> rocket::Response<'static> {
        use rocket::http::ContentType;
        use rocket::http::Status;
        use rocket::Response;
        use std::io::Cursor;

        let content_type: ContentType = PROBLEM_JSON_MEDIA_TYPE.parse().unwrap();
        let json = self.json_bytes();
        let response = Response::build()
            .status(Status {
                code: self.status_code_or_internal_server_error().into(),
            })
            .sized_body(json.len(), Cursor::new(json))
            .header(content_type)
            .finalize();

        response
    }

    /// Creates a [salvo] response.
    ///
    /// If status is `None` `500 - Internal Server Error` is the
    /// default.
    ///
    /// Requires the `salvo` feature
    #[cfg(feature = "salvo")]
    pub fn to_salvo_response(&self) -> salvo::Response {
        use salvo::hyper::header::{HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};
        use salvo::hyper::*;

        let json = self.json_bytes();
        let length = json.len() as u64;

        let (mut parts, body) = Response::new(json.into()).into_parts();

        parts.headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(PROBLEM_JSON_MEDIA_TYPE),
        );
        parts.headers.insert(
            CONTENT_LENGTH,
            HeaderValue::from_str(&length.to_string()).unwrap(),
        );
        parts.status = self.status_or_internal_server_error();

        Response::from_parts(parts, body).into()
    }

    /// Creates a [tide] response.
    ///
    /// If status is `None` `500 - Internal Server Error` is the
    /// default.
    ///
    /// Requires the `tide` feature
    #[cfg(feature = "tide")]
    pub fn to_tide_response(&self) -> tide::Response {
        let json = self.json_bytes();
        let length = json.len() as u64;

        tide::Response::builder(self.status_code_or_internal_server_error())
            .body(json)
            .header("Content-Length", length.to_string())
            .content_type(PROBLEM_JSON_MEDIA_TYPE)
            .build()
    }

    fn status_or_internal_server_error(&self) -> StatusCode {
        self.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn status_code_or_internal_server_error(&self) -> u16 {
        self.status_or_internal_server_error().as_u16()
    }

    // Deprecations

    #[deprecated(since = "0.50.0", note = "please use `with_title` instead")]
    pub fn with_title_from_status<T: Into<StatusCode>>(status: T) -> Self {
        Self::with_title(status)
    }
    #[deprecated(since = "0.50.0", note = "please use `with_title_and_type` instead")]
    pub fn with_title_and_type_from_status<T: Into<StatusCode>>(status: T) -> Self {
        Self::with_title_and_type(status)
    }
    #[deprecated(since = "0.50.0", note = "please use `status` instead")]
    pub fn set_status<T: Into<StatusCode>>(self, status: T) -> Self {
        self.status(status)
    }
    #[deprecated(since = "0.50.0", note = "please use `title` instead")]
    pub fn set_title<T: Into<String>>(self, title: T) -> Self {
        self.title(title)
    }
    #[deprecated(since = "0.50.0", note = "please use `detail` instead")]
    pub fn set_detail<T: Into<String>>(self, detail: T) -> Self {
        self.detail(detail)
    }
    #[deprecated(since = "0.50.0", note = "please use `type_url` instead")]
    pub fn set_type_url<T: Into<String>>(self, type_url: T) -> Self {
        self.type_url(type_url)
    }
    #[deprecated(since = "0.50.0", note = "please use `instance` instead")]
    pub fn set_instance<T: Into<String>>(self, instance: T) -> Self {
        self.instance(instance)
    }
}

impl fmt::Display for HttpApiProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(status) = self.status {
            write!(f, "{}", status)?;
        } else {
            write!(f, "<no status>")?;
        }

        match (self.title.as_ref(), self.detail.as_ref()) {
            (Some(title), Some(detail)) => return write!(f, " - {} - {}", title, detail),
            (Some(title), None) => return write!(f, " - {}", title),
            (None, Some(detail)) => return write!(f, " - {}", detail),
            (None, None) => (),
        }

        if let Some(type_url) = self.type_url.as_ref() {
            return write!(f, " - {}", type_url);
        }

        Ok(())
    }
}

impl Error for HttpApiProblem {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl From<StatusCode> for HttpApiProblem {
    fn from(status: StatusCode) -> HttpApiProblem {
        HttpApiProblem::new(status)
    }
}

/// Creates an [hyper::Response] from something that can become an
/// `HttpApiProblem`.
///
/// If status is `None` `500 - Internal Server Error` is the
/// default.
#[cfg(feature = "hyper")]
pub fn into_hyper_response<T: Into<HttpApiProblem>>(what: T) -> hyper::Response<hyper::Body> {
    let problem: HttpApiProblem = what.into();
    problem.to_hyper_response()
}

#[cfg(feature = "hyper")]
impl From<HttpApiProblem> for hyper::Response<hyper::Body> {
    fn from(problem: HttpApiProblem) -> hyper::Response<hyper::Body> {
        problem.to_hyper_response()
    }
}

// Creates an `actix::HttpResponse` from something that can become an
/// `HttpApiProblem`.
///
/// If status is `None` `500 - Internal Server Error` is the
/// default.
#[cfg(feature = "actix-web")]
pub fn into_actix_response<T: Into<HttpApiProblem>>(what: T) -> actix_web::HttpResponse {
    let problem: HttpApiProblem = what.into();
    problem.to_actix_response()
}

#[cfg(feature = "actix-web")]
impl From<HttpApiProblem> for actix_web::HttpResponse {
    fn from(problem: HttpApiProblem) -> actix_web::HttpResponse {
        problem.to_actix_response()
    }
}

/// Creates an `rocket::Response` from something that can become an
/// `HttpApiProblem`.
///
/// If status is `None` `500 - Internal Server Error` is the
/// default.
#[cfg(feature = "rocket")]
pub fn into_rocket_response<T: Into<HttpApiProblem>>(what: T) -> ::rocket::Response<'static> {
    let problem: HttpApiProblem = what.into();
    problem.to_rocket_response()
}

#[cfg(feature = "rocket")]
impl From<HttpApiProblem> for ::rocket::Response<'static> {
    fn from(problem: HttpApiProblem) -> ::rocket::Response<'static> {
        problem.to_rocket_response()
    }
}

#[cfg(feature = "rocket")]
impl<'r> ::rocket::response::Responder<'r, 'static> for HttpApiProblem {
    fn respond_to(self, _request: &::rocket::Request) -> ::rocket::response::Result<'static> {
        Ok(self.into())
    }
}

#[cfg(feature = "warp")]
impl warp::reject::Reject for HttpApiProblem {}

/// Creates a [salvo::Response] from something that can become an
/// `HttpApiProblem`.
///
/// If status is `None` `500 - Internal Server Error` is the
/// default.
#[cfg(feature = "salvo")]
pub fn into_salvo_response<T: Into<HttpApiProblem>>(what: T) -> salvo::Response {
    let problem: HttpApiProblem = what.into();
    problem.to_salvo_response()
}

#[cfg(feature = "salvo")]
impl From<HttpApiProblem> for salvo::Response {
    fn from(problem: HttpApiProblem) -> salvo::Response {
        problem.to_salvo_response()
    }
}

/// Creates a [tide::Response] from something that can become an
/// `HttpApiProblem`.
///
/// If status is `None` `500 - Internal Server Error` is the
/// default.
#[cfg(feature = "tide")]
pub fn into_tide_response<T: Into<HttpApiProblem>>(what: T) -> tide::Response {
    let problem: HttpApiProblem = what.into();
    problem.to_tide_response()
}

#[cfg(feature = "tide")]
impl From<HttpApiProblem> for tide::Response {
    fn from(problem: HttpApiProblem) -> tide::Response {
        problem.to_tide_response()
    }
}

mod custom_http_status_serialization {
    use http::StatusCode;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::convert::TryFrom;

    pub fn serialize<S>(status: &Option<StatusCode>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref status_code) = *status {
            return s.serialize_u16(status_code.as_u16());
        }
        s.serialize_none()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<StatusCode>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<u16> = Option::deserialize(deserializer)?;
        if let Some(numeric_status_code) = s {
            // If the status code numeral is invalid we simply return None.
            // This is a trade off to guarantee that the client can still
            // have access to the rest of the problem struct instead of
            // having to deal with an error caused by trying to deserialize an invalid status
            // code. Additionally the received response still contains a status code.
            let status_code = StatusCode::try_from(numeric_status_code).ok();
            return Ok(status_code);
        }

        Ok(None)
    }
}

#[cfg(test)]
mod test;
