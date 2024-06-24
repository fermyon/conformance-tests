use bindings::{
    exports::wasi::http0_2_0::incoming_handler::Guest,
    fermyon::spin2_0_0::sqlite::{Connection, Error, Value},
    wasi::http0_2_0::types::{
        Headers, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
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
        let result = match handle(request) {
            Ok(()) => response(200, b""),
            Err(e) => response(500, format!("{e}").as_bytes()),
        };

        ResponseOutparam::set(response_out, Ok(result))
    }
}

fn handle(_request: IncomingRequest) -> anyhow::Result<()> {
    anyhow::ensure!(matches!(
        Connection::open("forbidden"),
        Err(Error::AccessDenied)
    ));

    let conn = Connection::open("default")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS test_data(key TEXT NOT NULL, value TEXT NOT NULL);",
        &[],
    )?;

    conn.execute(
        "INSERT INTO test_data(key, value) VALUES('my_key', 'my_value');",
        &[],
    )?;

    let results = conn.execute(
        "SELECT * FROM test_data WHERE key = ?",
        &[Value::Text("my_key".to_owned())],
    )?;

    anyhow::ensure!(results.rows.len() == 1);
    anyhow::ensure!(results.columns.len() == 2);

    let key_index = results.columns.iter().position(|c| c == "key").unwrap();
    let value_index = results.columns.iter().position(|c| c == "value").unwrap();

    let fetched_key = &results.rows[0].values[key_index];
    let fetched_value = &results.rows[0].values[value_index];

    anyhow::ensure!(matches!(fetched_key, Value::Text(t) if t == "my_key"));
    anyhow::ensure!(matches!(fetched_value, Value::Text(t) if t == "my_value"));

    Ok(())
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
