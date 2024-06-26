use helper::bindings::wasi::{
    http0_2_0::types::{IncomingRequest, OutgoingResponse, ResponseOutparam},
    io0_2_0::poll,
    sockets0_2_0::{
        instance_network,
        network::{
            ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
        },
        tcp_create_socket,
    },
};

use std::net::SocketAddr;

helper::gen_http_trigger_bindings!(Component);

struct Component;

impl bindings::Guest for Component {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        helper::handle_result(handle(request), outparam);
    }
}

fn handle(request: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
    // The request must have a "url" header.
    let Some(address) = request.headers().entries().iter().find_map(|(k, v)| {
        (k == "address")
            .then_some(v)
            .and_then(|v| std::str::from_utf8(v).ok())
            .and_then(|v| v.parse().ok())
    }) else {
        // Otherwise, return a 400 Bad Request response.
        return Ok(helper::response(400, b"Bad Request"));
    };
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
    Ok(helper::ok_response())
}
