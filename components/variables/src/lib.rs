use bindings::{
    exports::wasi::http0_2_0::incoming_handler::Guest,
    fermyon::spin2_0_0::variables::{get, Error},
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
    anyhow::ensure!(matches!(get("variable"), Ok(val) if val == "value"));
    anyhow::ensure!(matches!(get("non_existent"), Err(Error::Undefined(_))));

    anyhow::ensure!(matches!(get("invalid-name"), Err(Error::InvalidName(_))));
    anyhow::ensure!(matches!(get("invalid!name"), Err(Error::InvalidName(_))));
    anyhow::ensure!(matches!(get("4invalidname"), Err(Error::InvalidName(_))));

    Ok(())
}
