use anyhow::Context as _;

struct Component;

helper::gen_http_trigger_bindings!(Component);

use helper::bindings::{
    fermyon::spin2_0_0::key_value::{Error, Store},
    wasi::http0_2_0::types::{IncomingRequest, OutgoingResponse, ResponseOutparam},
};

impl bindings::Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        helper::handle_result(handle(request), response_out);
    }
}

fn handle(_req: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
    anyhow::ensure!(matches!(Store::open("forbidden"), Err(Error::AccessDenied)));
    let store = Store::open("default")?;

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

    Ok(helper::ok_response())
}
