use anyhow::Context as _;
use bindings::{
    exports::wasi::http0_2_0::incoming_handler::Guest,
    fermyon::spin2_0_0::redis,
    wasi::http0_2_0::types::{
        Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
    },
};

struct Component;

mod bindings {
    wit_bindgen::generate!({
            world: "http-trigger",
            path:  "../../wit",
    });
    use super::Component;
    export!(Component);
}

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let result = match handle(request) {
            Err(e) => response(500, format!("{e}").as_bytes()),
            Ok(r) => r,
        };
        ResponseOutparam::set(response_out, Ok(result))
    }
}

const REDIS_ADDRESS_HEADER: &str = "REDIS_ADDRESS";

fn handle(request: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
    let Some(address) = request
        .headers()
        .get(&REDIS_ADDRESS_HEADER.to_owned())
        .pop()
        .and_then(|v| String::from_utf8(v).ok())
    else {
        // Otherwise, return a 400 Bad Request response.
        return Ok(response(400, b"Bad Request"));
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

    connection.execute(
        "set",
        &[
            redis::RedisParameter::Binary(b"int-key".to_vec()),
            redis::RedisParameter::Int64(0),
        ],
    )?;

    let values = connection.execute(
        "incr",
        &[redis::RedisParameter::Binary(b"int-key".to_vec())],
    )?;

    anyhow::ensure!(
        matches!(values.as_slice(), &[redis::RedisResult::Int64(1)]),
        "call to `execute('incr')` returned unexpected result: {values:?} != &[redis::RedisResult::Int64(1)]"
    );

    let values =
        connection.execute("get", &[redis::RedisParameter::Binary(b"int-key".to_vec())])?;

    anyhow::ensure!(matches!(
        values.as_slice(),
        &[redis::RedisResult::Binary(ref b)] if b == b"1"
    ));

    connection.execute("del", &[redis::RedisParameter::Binary(b"foo".to_vec())])?;

    connection.execute(
        "sadd",
        &[
            redis::RedisParameter::Binary(b"foo".to_vec()),
            redis::RedisParameter::Binary(b"bar".to_vec()),
            redis::RedisParameter::Binary(b"baz".to_vec()),
        ],
    )?;

    let values = connection.execute(
        "smembers",
        &[redis::RedisParameter::Binary(b"foo".to_vec())],
    )?;
    let mut values = values
        .iter()
        .map(|v| match v {
            redis::RedisResult::Binary(v) => Ok(v.as_slice()),
            v => Err(anyhow::anyhow!("unexpected value: {v:?}")),
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    // Ensure the values are always in a deterministic order
    values.sort();

    anyhow::ensure!(matches!(values.as_slice(), &[b"bar", b"baz",]));

    connection.execute(
        "srem",
        &[
            redis::RedisParameter::Binary(b"foo".to_vec()),
            redis::RedisParameter::Binary(b"baz".to_vec()),
        ],
    )?;

    let values = connection.execute(
        "smembers",
        &[redis::RedisParameter::Binary(b"foo".to_vec())],
    )?;

    anyhow::ensure!(matches!(
        values.as_slice(),
        &[redis::RedisResult::Binary(ref bar)] if bar == b"bar"
    ));

    Ok(response(200, b""))
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
