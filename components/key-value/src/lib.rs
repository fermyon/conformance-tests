use anyhow::Context;
use bindings::{
    exports::wasi::http0_2_0::incoming_handler::Guest,
    fermyon::spin2_0_0::key_value::{Error, Store},
    wasi::http0_2_0::types::{
        ErrorCode, Headers, IncomingRequest, OutgoingResponse, ResponseOutparam,
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
            .map(|_| OutgoingResponse::new(Headers::new()))
            .map_err(|e| ErrorCode::InternalError(Some(e.to_string())));
        ResponseOutparam::set(response_out, result)
    }
}

fn handle(_req: IncomingRequest) -> anyhow::Result<()> {
    anyhow::ensure!(matches!(Store::open("forbidden"), Err(Error::AccessDenied)));

    let store = Store::open("default").context("could not open 'default' store")?;

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

    Ok(())
}
