//! Helper functions for the conformance test components

/// Bindings to all imports
pub mod bindings {
    wit_bindgen::generate!({
        world: "platform",
        path:  "../../wit",
    });
}

/// Generate bindings for the http-trigger world.
#[macro_export]
macro_rules! gen_http_trigger_bindings {
    ($ident:ident) => {
        mod bindings {
            wit_bindgen::generate!({
                 world: "http-trigger",
                 path:  "../../wit",
                 with: {
                     "wasi:http/types@0.2.0": helper::bindings::wasi::http0_2_0::types,
                     "wasi:http/outgoing-handler@0.2.0": helper::bindings::wasi::http0_2_0::outgoing_handler,
                 }
            });
            use super::$ident;
            export!($ident);

            pub use exports::wasi::http0_2_0::incoming_handler::Guest;
        }

    };
}

use bindings::wasi::http0_2_0::types::{
    Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

/// Create a response with the given status code and body.
pub fn response(status: u16, body: &[u8]) -> OutgoingResponse {
    let response = OutgoingResponse::new(Headers::new());
    response.set_status_code(status).unwrap();
    if !body.is_empty() {
        write_outgoing_body(response.body().unwrap(), body);
    }
    response
}

/// Write the given content to the outgoing body.
pub fn write_outgoing_body(outgoing_body: OutgoingBody, content: &[u8]) {
    assert!(content.len() <= 4096);
    {
        let outgoing_stream = outgoing_body.write().unwrap();
        outgoing_stream.blocking_write_and_flush(content).unwrap();
        // The outgoing stream must be dropped before the outgoing body is finished.
    }
    OutgoingBody::finish(outgoing_body, None).unwrap();
}

/// Return a 500 response if the result is an error, otherwise return the response.
pub fn handle_result(result: anyhow::Result<OutgoingResponse>, response_out: ResponseOutparam) {
    let result = match result {
        Err(e) => response(500, format!("{e}").as_bytes()),
        Ok(response) => response,
    };
    ResponseOutparam::set(response_out, Ok(result))
}

/// Get the value of a header from a request.
pub fn get_header(request: &IncomingRequest, header_key: &String) -> Option<String> {
    request
        .headers()
        .get(header_key)
        .pop()
        .and_then(|v| String::from_utf8(v).ok())
}

/// Create a response with a 200 status code and an empty body.
pub fn ok_response() -> OutgoingResponse {
    response(200, b"")
}
