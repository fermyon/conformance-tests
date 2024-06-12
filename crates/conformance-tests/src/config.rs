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

impl Request {
    /// Send the request
    pub fn send<F>(self, send: F) -> anyhow::Result<test_environment::http::Response>
    where
        F: for<'a, 'b> FnOnce(
            test_environment::http::Request<'a, String>,
        ) -> anyhow::Result<test_environment::http::Response>,
    {
        let headers = self
            .headers
            .iter()
            .map(|h| (h.name.as_str(), h.value.as_str()))
            .collect::<Vec<_>>();
        let request = test_environment::http::Request::full(
            match self.method {
                Method::GET => test_environment::http::Method::Get,
                Method::POST => test_environment::http::Method::Post,
            },
            &self.path,
            &headers,
            self.body,
        );
        send(request)
    }
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
