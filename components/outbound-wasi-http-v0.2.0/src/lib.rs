use helper::bindings::wasi::http0_2_0::outgoing_handler;
use helper::bindings::wasi::http0_2_0::types::{
    Headers, IncomingRequest, Method, OutgoingBody, OutgoingRequest, OutgoingResponse,
    ResponseOutparam, Scheme,
};
use helper::bindings::wasi::io0_2_0::streams::StreamError;
use url::Url;

struct Component;

helper::gen_http_trigger_bindings!(Component);

impl bindings::Guest for Component {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        // The request must have a "url" header.
        let Some(url) = request.headers().entries().iter().find_map(|(k, v)| {
            (k == "url")
                .then_some(v)
                .and_then(|v| std::str::from_utf8(v).ok())
                .and_then(|v| Url::parse(v).ok())
        }) else {
            // Otherwise, return a 400 Bad Request response.
            return_response(outparam, 400, b"missing header: url");
            return;
        };

        let headers = Headers::new();
        headers
            .append(&"Content-Length".into(), &"13".into())
            .unwrap();
        let outgoing_request = OutgoingRequest::new(headers);
        outgoing_request.set_method(&Method::Post).unwrap();
        outgoing_request
            .set_path_with_query(Some(url.path()))
            .unwrap();
        outgoing_request
            .set_scheme(Some(&match url.scheme() {
                "http" => Scheme::Http,
                "https" => Scheme::Https,
                scheme => Scheme::Other(scheme.into()),
            }))
            .unwrap();
        outgoing_request
            .set_authority(Some(url.authority()))
            .unwrap();

        // Write the request body.
        helper::write_outgoing_body(outgoing_request.body().unwrap(), b"Hello, world!");

        // Get the incoming response.
        let response = match outgoing_handler::handle(outgoing_request, None) {
            Ok(r) => r,
            Err(e) => {
                return_response(outparam, 500, e.to_string().as_bytes());
                return;
            }
        };

        let response = loop {
            if let Some(response) = response.get() {
                break response.unwrap().unwrap();
            } else {
                response.subscribe().block()
            }
        };
        let incoming_body = response.consume().unwrap();
        let incoming_stream = incoming_body.stream().unwrap();
        let status = response.status();

        // Create the outgoing response from the incoming response.
        let response = OutgoingResponse::new(response.headers().clone());
        response.set_status_code(status).unwrap();
        let outgoing_body = response.body().unwrap();
        {
            let outgoing_stream = outgoing_body.write().unwrap();
            ResponseOutparam::set(outparam, Ok(response));

            loop {
                match incoming_stream.blocking_read(1024) {
                    Ok(buffer) => {
                        outgoing_stream.blocking_write_and_flush(&buffer).unwrap();
                    }
                    Err(StreamError::Closed) => break,
                    Err(StreamError::LastOperationFailed(error)) => {
                        panic!("{}", error.to_debug_string())
                    }
                }
            }
            // The outgoing stream must be dropped before the outgoing body is finished.
        }

        OutgoingBody::finish(outgoing_body, None).unwrap();
    }
}

fn return_response(outparam: ResponseOutparam, status: u16, body: &[u8]) {
    let response = helper::response(status, body);
    ResponseOutparam::set(outparam, Ok(response));
}
