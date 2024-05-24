/// The configuration of a conformance test
#[derive(Debug, serde::Deserialize)]
pub struct TestConfig {
    pub invocations: Vec<Invocation>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Invocation {
    Http(HttpInvocation),
}

/// An invocation of the runtime
#[derive(Debug, serde::Deserialize)]
pub struct HttpInvocation {
    pub request: Request,
    pub response: Response,
}

#[derive(Debug, serde::Deserialize)]
pub struct Request {
    #[serde(default)]
    pub method: Method,
    pub path: String,
    #[serde(default)]
    pub headers: Vec<RequestHeader>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Response {
    #[serde(default = "default_response_status")]
    pub status: u16,
    pub headers: Vec<ResponseHeader>,
    pub body: Option<String>,
}

fn default_response_status() -> u16 {
    200
}

#[derive(Debug, serde::Deserialize)]
pub struct RequestHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ResponseHeader {
    pub name: String,
    pub value: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, serde::Deserialize, Default)]
pub enum Method {
    #[default]
    GET,
    POST,
}
