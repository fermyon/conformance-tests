use anyhow::Context;
use std::collections::HashMap;

mod bindings {
    wit_bindgen::generate!({
            world: "http-trigger",
            path:  "../../wit",
    });
    use super::Component;
    export!(Component);
}

use bindings::{
    exports::wasi::http0_2_0::incoming_handler::{Guest, IncomingRequest, ResponseOutparam},
    wasi::{
        cli0_2_0::stdout::get_stdout,
        http0_2_0::types::{ErrorCode, Headers, OutgoingResponse},
    },
};

use crate::bindings::wasi::http0_2_0::types::{Method, Scheme};

pub struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let result = handle(request)
            .map(|_| OutgoingResponse::new(Headers::new()))
            .map_err(|e| ErrorCode::InternalError(Some(e.to_string())));
        get_stdout()
            .blocking_write_and_flush(format!("Test Result: {result:?}\n").as_bytes())
            .unwrap();
        ResponseOutparam::set(response_out, result)
    }
}

fn handle(req: IncomingRequest) -> anyhow::Result<()> {
    check_method(&req)?;
    check_url(&req)?;
    check_headers(&req)?;
    Ok(())
}

fn check_method(req: &IncomingRequest) -> anyhow::Result<()> {
    let method = req.method();
    let expected = Method::Get;
    anyhow::ensure!(
        matches!(method, Method::Get),
        "Method was expected to be '{expected:?}' but was '{method:?}'"
    );

    Ok(())
}

fn check_url(req: &IncomingRequest) -> anyhow::Result<()> {
    let authority = req
        .authority()
        .context("incoming request has no authority")?;
    let _addr: std::net::SocketAddr = authority
        .parse()
        .context("authority is not a valid SocketAddr")?;

    let path_with_query = req.path_with_query();
    let expected = "/base/path/end/rest?key=value";
    anyhow::ensure!(
        path_with_query.as_deref() == Some(expected),
        "URL was expected to be '{expected}' but was '{path_with_query:?}'"
    );

    let scheme = req.scheme();
    let expected = Scheme::Http;
    anyhow::ensure!(
        matches!(scheme, Some(Scheme::Http)),
        "Scheme was expected to be '{expected:?}' but was '{scheme:?}'"
    );

    Ok(())
}

/// Check that the headers are as expected
fn check_headers(req: &IncomingRequest) -> anyhow::Result<()> {
    let expected_headers = [
        ("spin-raw-component-route", "/:path_segment/:path_end/..."),
        (
            "spin-full-url",
            "http://example.com/base/path/end/rest?key=value",
        ),
        ("spin-path-info", "/rest"),
        ("spin-base-path", "/base"),
        ("spin-component-route", "/:path_segment/:path_end"),
        ("spin-path-match-path-segment", "path"),
        ("spin-path-match-path-end", "end"),
        ("spin-matched-route", "/base/:path_segment/:path_end/..."),
    ];

    let mut actual_headers: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
    for (k, v) in req.headers().entries() {
        actual_headers.entry(k).or_default().push(v);
    }

    for (name, value) in expected_headers.into_iter() {
        let header = header_as_string(&mut actual_headers, name)?;

        anyhow::ensure!(
            header == value,
            "Header {name} was expected to contain value '{value}' but contained '{header}' "
        );
    }

    // Check that the spin-client-addr header is a valid SocketAddr
    let _: std::net::SocketAddr = header_as_string(&mut actual_headers, "spin-client-addr")?
        .parse()
        .context("spin-client-addr header is not a valid SocketAddr")?;

    // Check that there are no unexpected `spin-*` headers
    for (name, _) in actual_headers {
        let lowercase = &name.to_lowercase();
        if lowercase.starts_with("spin-") || lowercase.starts_with("spin_") {
            anyhow::bail!("unexpected special `spin-*` header '{name}' found in request");
        }
    }

    Ok(())
}

/// Fails unless there is exactly one header with the given name, and it is valid UTF-8
fn header_as_string(
    headers: &mut HashMap<String, Vec<Vec<u8>>>,
    name: &str,
) -> anyhow::Result<String> {
    //TODO: handle the fact that headers are case sensitive
    let mut value = headers.remove(name).unwrap_or_default();

    if value.len() != 1 {
        anyhow::bail!(
            "expected exactly one header '{name}' but found {}",
            value.len()
        )
    }
    String::from_utf8(value.remove(0))
        .with_context(|| format!("header '{name}' is not valid UTF-8"))
}
