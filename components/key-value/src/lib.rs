use anyhow::Context;
use bindings::{
    exports::wasi::http0_2_0::incoming_handler::Guest,
    fermyon::spin2_0_0::key_value::{Error, Store},
    wasi::http0_2_0::types::{
        ErrorCode, Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
    },
};

mod bindings {
    wit_bindgen::generate!({
            world: "http-trigger",
            path:  "../../wit",
    });
    use super::Component;
    export!(Component);
}

struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let result = handle(request)
            .map(|r| match r {
                Err(e) => response(500, format!("{e}").as_bytes()),
                Ok(()) => response(200, b""),
            })
            .map_err(|e| ErrorCode::InternalError(Some(e.to_string())));
        ResponseOutparam::set(response_out, result)
    }
}

fn handle(_req: IncomingRequest) -> anyhow::Result<Result<(), Error>> {
    anyhow::ensure!(matches!(Store::open("forbidden"), Err(Error::AccessDenied)));
    let store = match Store::open("default") {
        Ok(s) => s,
        Err(e) => return Ok(Err(e)),
    };

    // Ensure nothing set in `bar` key
    store.delete("bar").context("could not delete 'bar' key")?;
    anyhow::ensure!(matches!(store.exists("bar"), Ok(false)));
    anyhow::ensure!(matches!(store.get("bar"), Ok(None)));
    anyhow::ensure!(matches!(store.get_keys().as_deref(), Ok(&[])));

    // Set `bar` key
    store
        .set("bar", b"baz")
        .context("could not set 'bar' key")?;
    anyhow::ensure!(matches!(store.exists("bar"), Ok(true)));
    anyhow::ensure!(matches!(store.get("bar"), Ok(Some(v)) if v == b"baz"));
    anyhow::ensure!(matches!(store.get_keys().as_deref(), Ok([bar]) if bar == "bar"));

    // Override `bar` key
    store.set("bar", b"wow")?;
    anyhow::ensure!(matches!(store.exists("bar"), Ok(true)));
    anyhow::ensure!(matches!(store.get("bar"), Ok(Some(wow)) if wow == b"wow"));
    anyhow::ensure!(matches!(store.get_keys().as_deref(), Ok([bar]) if bar == "bar"));

    // Set another key
    store.set("qux", b"yay")?;
    anyhow::ensure!(
        matches!(store.get_keys().as_deref(), Ok(c) if c.len() == 2 && c.contains(&"bar".into()) && c.contains(&"qux".into()))
    );

    // Delete everything
    store.delete("bar")?;
    store.delete("bar")?;
    store.delete("qux")?;
    anyhow::ensure!(matches!(store.exists("bar"), Ok(false)));
    anyhow::ensure!(matches!(store.get("qux"), Ok(None)));
    anyhow::ensure!(matches!(store.get_keys().as_deref(), Ok(&[])));

    Ok(Ok(()))
}

fn response(status: u16, body: &[u8]) -> OutgoingResponse {
    let response = OutgoingResponse::new(Headers::new());
    response.set_status_code(status).unwrap();
    if !body.is_empty() {
        assert!(body.len() <= 4096);
        let outgoing_body = response.body().unwrap();
        {
            let outgoing_stream = outgoing_body.write().unwrap();
            outgoing_stream.blocking_write_and_flush(body).unwrap();
            // The outgoing stream must be dropped before the outgoing body is finished.
        }
        OutgoingBody::finish(outgoing_body, None).unwrap();
    }
    response
}
