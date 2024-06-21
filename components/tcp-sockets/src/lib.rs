use bindings::{
    exports::wasi::http0_2_0::incoming_handler::Guest,
    wasi::{
        http0_2_0::types::{
            Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
        },
        io0_2_0::poll,
        sockets0_2_0::{
            instance_network,
            network::{
                ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
            },
            tcp_create_socket,
        },
    },
};

use std::net::SocketAddr;

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
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        // The request must have a "url" header.
        let Some(address) = request.headers().entries().iter().find_map(|(k, v)| {
            (k == "address")
                .then_some(v)
                .and_then(|v| std::str::from_utf8(v).ok())
                .and_then(|v| v.parse().ok())
        }) else {
            // Otherwise, return a 400 Bad Request response.
            return_response(outparam, 400, b"Bad Request");
            return;
        };

        match make_request(address) {
            Ok(()) => return_response(outparam, 200, b""),
            Err(e) => return_response(outparam, 500, format!("{e}").as_bytes()),
        }
    }
}

fn make_request(address: SocketAddr) -> anyhow::Result<()> {
    let client = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4)?;

    client.start_connect(
        &instance_network::instance_network(),
        match address {
            SocketAddr::V6(address) => {
                let ip = address.ip().segments();
                IpSocketAddress::Ipv6(Ipv6SocketAddress {
                    address: ip.into(),
                    port: address.port(),
                    flow_info: 0,
                    scope_id: 0,
                })
            }
            SocketAddr::V4(address) => {
                let ip = address.ip().octets();
                IpSocketAddress::Ipv4(Ipv4SocketAddress {
                    address: ip.into(),
                    port: address.port(),
                })
            }
        },
    )?;

    let (rx, tx) = loop {
        match client.finish_connect() {
            Err(ErrorCode::WouldBlock) => {
                poll::poll(&[&client.subscribe()]);
            }
            result => break result?,
        }
    };

    let message = b"So rested he by the Tumtum tree";
    tx.blocking_write_and_flush(message)?;

    let mut buffer = Vec::with_capacity(message.len());
    while buffer.len() < message.len() {
        let chunk = rx.blocking_read((message.len() - buffer.len()).try_into().unwrap())?;
        buffer.extend(chunk);
    }
    assert_eq!(buffer.as_slice(), message);
    Ok(())
}

fn write_outgoing_body(outgoing_body: OutgoingBody, message: &[u8]) {
    assert!(message.len() <= 4096);
    {
        let outgoing_stream = outgoing_body.write().unwrap();
        outgoing_stream.blocking_write_and_flush(message).unwrap();
        // The outgoing stream must be dropped before the outgoing body is finished.
    }
    OutgoingBody::finish(outgoing_body, None).unwrap();
}

fn return_response(outparam: ResponseOutparam, status: u16, body: &[u8]) {
    let response = OutgoingResponse::new(Headers::new());
    response.set_status_code(status).unwrap();
    write_outgoing_body(response.body().unwrap(), body);

    ResponseOutparam::set(outparam, Ok(response));
}
