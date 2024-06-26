use helper::bindings::{
    fermyon::spin2_0_0::sqlite::{Connection, Error, Value},
    wasi::http0_2_0::types::{IncomingRequest, OutgoingResponse, ResponseOutparam},
};

struct Component;

helper::gen_http_trigger_bindings!(Component);

impl bindings::Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        helper::handle_result(handle(request), response_out)
    }
}

fn handle(_request: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
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

    Ok(helper::ok_response())
}
