use anyhow::Context as _;

struct Component;

helper::gen_http_trigger_bindings!(Component);
use bindings::{exports::wasi::http0_2_0::incoming_handler::Guest, fermyon::spin2_0_0::redis};
use helper::bindings::wasi::http0_2_0::types::{
    IncomingRequest, OutgoingResponse, ResponseOutparam,
};

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        helper::handle_result(handle(request), response_out);
    }
}

const REDIS_ADDRESS_HEADER: &str = "REDIS_ADDRESS";

fn handle(request: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
    let Some(address) = helper::get_header(request, &REDIS_ADDRESS_HEADER.to_owned()) else {
        // Otherwise, return a 400 Bad Request response.
        return Ok(helper::response(
            400,
            format!("missing header: {REDIS_ADDRESS_HEADER}").as_bytes(),
        ));
    };
    let connection = redis::Connection::open(&address)?;

    connection.set("spin-example-get-set", &b"Eureka!".to_vec())?;

    let payload = connection
        .get("spin-example-get-set")?
        .context("missing value for 'spin-example-get-set'")?;

    anyhow::ensure!(String::from_utf8_lossy(&payload) == "Eureka!");

    connection.set("spin-example-incr", &b"0".to_vec())?;

    let int_value = connection.incr("spin-example-incr")?;

    anyhow::ensure!(int_value == 1);

    let keys = vec!["spin-example-get-set".into(), "spin-example-incr".into()];

    let del_keys = connection.del(&keys)?;

    anyhow::ensure!(del_keys == 2);

    connection.execute(
        "set",
        &[
            redis::RedisParameter::Binary(b"spin-example".to_vec()),
            redis::RedisParameter::Binary(b"Eureka!".to_vec()),
        ],
    )?;

    connection.execute(
        "append",
        &[
            redis::RedisParameter::Binary(b"spin-example".to_vec()),
            redis::RedisParameter::Binary(b" I've got it!".to_vec()),
        ],
    )?;

    let values = connection.execute(
        "get",
        &[redis::RedisParameter::Binary(b"spin-example".to_vec())],
    )?;

    anyhow::ensure!(matches!(
        values.as_slice(),
        &[redis::RedisResult::Binary(ref b)] if b == b"Eureka! I've got it!"));

    Ok(helper::ok_response())
}
