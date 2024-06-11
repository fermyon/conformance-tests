//! Utilities for tests that function over HTTP

use std::collections::HashMap;

use anyhow::Context as _;

/// A request to the Spin server
#[derive(Debug, Clone)]
pub struct Request<'a, B> {
    pub method: Method,
    pub path: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub body: Option<B>,
}

impl<'a, 'b> Request<'a, &'b [u8]> {
    /// Create a new request with no headers or body
    pub fn new(method: Method, uri: &'a str) -> Self {
        Self {
            method,
            path: uri,
            headers: &[],
            body: None,
        }
    }
}

impl<'a, B> Request<'a, B> {
    /// Create a new request with headers and a body
    pub fn full(
        method: Method,
        path: &'a str,
        headers: &'a [(&'a str, &'a str)],
        body: Option<B>,
    ) -> Self {
        Self {
            method,
            path,
            headers,
            body,
        }
    }
}

impl<'a, B: Into<reqwest::Body>> Request<'a, B> {
    /// Send the request to the given host and port
    pub fn send(self, host: &str, port: u16) -> anyhow::Result<Response> {
        let mut outgoing = reqwest::Request::new(
            self.method.into(),
            reqwest::Url::parse(&format!("http://{host}:{port}"))
                .unwrap()
                .join(self.path)
                .context("could not construct url for request against Spin")?,
        );
        outgoing
            .headers_mut()
            .extend(self.headers.iter().map(|(k, v)| {
                (
                    reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    reqwest::header::HeaderValue::from_str(v).unwrap(),
                )
            }));
        *outgoing.body_mut() = self.body.map(Into::into);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let mut retries = 0;
            let mut response = loop {
                let Some(request) = outgoing.try_clone() else {
                    break reqwest::Client::new().execute(outgoing).await;
                };
                let response = reqwest::Client::new().execute(request).await;
                if response.is_err() && retries < 5 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    retries += 1;
                } else {
                    break response;
                }
            }?;
            let mut chunks = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                chunks.push(chunk.to_vec());
            }
            Result::<_, anyhow::Error>::Ok(Response::full(
                response.status().as_u16(),
                response
                    .headers()
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k.as_str().to_owned(),
                            v.to_str().unwrap_or("<non-utf8>").to_owned(),
                        )
                    })
                    .collect(),
                chunks,
            ))
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Into<reqwest::Method> for Method {
    fn into(self) -> reqwest::Method {
        match self {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
        }
    }
}

/// A response from a Spin server
pub struct Response {
    status: u16,
    headers: HashMap<String, String>,
    chunks: Vec<Vec<u8>>,
}

impl Response {
    /// A response with no headers or body
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Default::default(),
            chunks: Default::default(),
        }
    }

    /// A response with headers and a body
    pub fn new_with_body(status: u16, chunks: impl IntoChunks) -> Self {
        Self {
            status,
            headers: Default::default(),
            chunks: chunks.into_chunks(),
        }
    }

    /// A response with headers and a body
    pub fn full(status: u16, headers: HashMap<String, String>, chunks: impl IntoChunks) -> Self {
        Self {
            status,
            headers,
            chunks: chunks.into_chunks(),
        }
    }

    /// The status code of the response
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The headers of the response
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// The body of the response
    pub fn body(&self) -> Vec<u8> {
        self.chunks.iter().flatten().copied().collect()
    }

    /// The body of the response as chunks of bytes
    ///
    /// If the response is not stream this will be a single chunk equal to the body
    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }

    /// The body of the response as a string
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body())
    }
}

pub trait IntoChunks {
    fn into_chunks(self) -> Vec<Vec<u8>>;
}

impl IntoChunks for Vec<Vec<u8>> {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        self
    }
}

impl IntoChunks for Vec<u8> {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self]
    }
}

impl IntoChunks for String {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self.into_bytes()]
    }
}

impl IntoChunks for &str {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self.as_bytes().into()]
    }
}
