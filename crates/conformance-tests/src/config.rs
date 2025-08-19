use anyhow::Context as _;
use std::path::Path;

/// Parse the test configuration from a file
pub fn parse_from_file(path: impl AsRef<Path>) -> anyhow::Result<TestConfig> {
    let config = std::fs::read_to_string(path).context("failed to read test manifest")?;
    parse(&config)
}

/// Parse the test configuration
pub fn parse(config: &str) -> anyhow::Result<TestConfig> {
    json5::from_str::<TestConfig>(config).context("test config could not be parsed")
}

/// The configuration of a conformance test
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestConfig {
    pub invocations: Vec<Invocation>,
    #[serde(default)]
    pub preconditions: Vec<Precondition>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum Invocation {
    Http(HttpInvocation),
}

/// An invocation of the runtime
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HttpInvocation {
    pub request: Request,
    pub response: Response,
}

impl HttpInvocation {
    /// Run the invocation by sending the request and asserting the response
    pub fn run<F>(self, send: F) -> anyhow::Result<test_environment::http::Response>
    where
        F: for<'a, 'b> FnOnce(
            test_environment::http::Request<'a, String>,
        ) -> anyhow::Result<test_environment::http::Response>,
    {
        self.request.send(|request| {
            let response = send(request).context("failed to send the request to the runtime")?;
            crate::assertions::assert_response(&self.response, &response)
                .context("assertion failed")?;
            Ok(response)
        })
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
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
    /// Substitute template variables in the request with well known env variables
    ///
    /// Supported variables:
    /// - port: map a well known guest port to the port exposed by the service on the host
    pub fn substitute_from_env<R>(
        &mut self,
        env: &mut test_environment::TestEnvironment<R>,
    ) -> anyhow::Result<()> {
        self.substitute(move |key, value| {
            if key != "port" {
                anyhow::bail!("unknown template key: {key}")
            }
            let port = env
                .get_port(value.parse().context("port must be a number")?)?
                .with_context(|| format!("no port {value} exposed by any service"))?;
            Ok(Some(port.to_string()))
        })
    }

    /// Substitute template variables in the request
    pub fn substitute(
        &mut self,
        mut replacement: impl FnMut(&str, &str) -> anyhow::Result<Option<String>>,
    ) -> anyhow::Result<()> {
        for header in &mut self.headers {
            test_environment::manifest_template::replace_template(
                &mut header.value,
                &mut replacement,
            )?;
        }
        Ok(())
    }

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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Response {
    #[serde(default = "default_response_status")]
    pub status: u16,
    pub headers: Vec<ResponseHeader>,
    pub body: Option<String>,
}

fn default_response_status() -> u16 {
    200
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RequestHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ResponseHeader {
    pub name: String,
    pub value: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub enum Method {
    #[default]
    GET,
    POST,
}

/// A precondition that must be met before the test can be run
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Precondition {
    /// A precondition that the key-value store must exist
    KeyValueStore(KeyValueStorePrecondition),
    /// The test expects outgoing HTTP requests to be echoed back
    ///
    /// The test runner should start an HTTP server that echoes back the request
    /// and it should update any references that test assets make to port 80 to
    /// the port of the echo server.
    HttpEcho,
    /// The test expects outgoing TCP requests to be echoed back
    ///
    /// The test runner should start a TCP server that echoes back the request
    /// and it should update any references that test assets make to port 5000 to
    /// the port of the echo server.
    TcpEcho,
    /// The test expects a sqlite service to be available.
    Sqlite,
    /// The test expects a Redis service to be available.
    Redis,
    /// The test expects a MQTT service to be available.
    Mqtt,
    /// The test expects a PostgreSQL service to be available.
    Postgres,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyValueStorePrecondition {
    pub label: String,
}
