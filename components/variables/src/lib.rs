use helper::bindings::{
    fermyon::spin2_0_0::variables::{get, Error},
    wasi::http0_2_0::types::{IncomingRequest, OutgoingResponse, ResponseOutparam},
};

struct Component;

helper::gen_http_trigger_bindings!(Component);

impl bindings::Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        helper::handle_result(handle(request), response_out);
    }
}

fn handle(_req: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
    anyhow::ensure!(matches!(get("variable"), Ok(val) if val == "value"));
    anyhow::ensure!(matches!(get("non_existent"), Err(Error::Undefined(_))));

    anyhow::ensure!(matches!(get("invalid-name"), Err(Error::InvalidName(_))));
    anyhow::ensure!(matches!(get("invalid!name"), Err(Error::InvalidName(_))));
    anyhow::ensure!(matches!(get("4invalidname"), Err(Error::InvalidName(_))));

    Ok(helper::ok_response())
}
