use anyhow::Context;

/// The configuration of a conformance test
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TestConfig {
    pub invocations: Vec<Invocation>,
    #[serde(default)]
    pub services: Vec<String>,
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
            let response = send(request)?;
            crate::assertions::assert_response(&self.response, &response)?;
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
            if key == "port" {
                let port = env.get_port(value.parse().context("port must be a number")?)?;
                match port {
                    Some(port) => Ok(Some(port.to_string())),
                    None => anyhow::bail!("no port {value} exposed by any service"),
                }
            } else {
                Ok(None)
            }
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
