#![allow(clippy::result_large_err)] // Clippy thinks the PgError type makes Results too big

use anyhow::{ensure, Context as _};
use helper::bindings::{
    spin::postgres4_0_0::postgres::{
        Connection, DbValue, Error as PgError, ParameterValue, RowSet,
    },
    wasi::http0_2_0::types::{IncomingRequest, OutgoingResponse, ResponseOutparam},
};
use rust_decimal::Decimal;

struct Component;
helper::gen_http_trigger_bindings!(Component);

// Format: "host=localhost port=5432 user=postgres password=postgres dbname=spin_dev"
const PG_CONNECTION_STRING: &str = "PG_CONNECTION_STRING";

impl bindings::Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        helper::handle_result(handle(request), response_out)
    }
}

fn handle(request: IncomingRequest) -> anyhow::Result<OutgoingResponse> {
    let conn_str = get_header(&request, PG_CONNECTION_STRING)?;
    let conn = Connection::open(&conn_str)?;

    test_numeric_types(&conn)?;

    let rowset = character_types(&conn)?;
    ensure!(rowset.rows.iter().all(|r| r.len() == 3));
    ensure!(matches!(rowset.rows[0][0], DbValue::Str(ref s) if s == "rvarchar"));

    let rowset = date_time_types(&conn)?;
    ensure!(rowset.rows.iter().all(|r| r.len() == 4));
    ensure!(
        matches!(rowset.rows[0][1], DbValue::Date((y, m, d)) if y == 2525 && m == 12 && d == 25)
    );
    ensure!(
        matches!(rowset.rows[0][2], DbValue::Time((h, m, s, ns)) if h == 4 && m == 5 && s == 6 && ns == 789_000_000)
    );
    ensure!(
        matches!(rowset.rows[0][3], DbValue::Datetime((y, _, _, h, _, _, ns)) if y == 1989 && h == 1 && ns == 0)
    );
    ensure!(
        matches!(rowset.rows[1][1], DbValue::Date((y, m, d)) if y == 2525 && m == 12 && d == 25)
    );
    ensure!(
        matches!(rowset.rows[1][2], DbValue::Time((h, m, s, ns)) if h == 14 && m == 15 && s == 16 && ns == 17)
    );
    ensure!(
        matches!(rowset.rows[1][3], DbValue::Datetime((y, _, _, h, _, _, ns)) if y == 1989 && h == 1 && ns == 4)
    );

    let rowset = json_types(&conn)?;
    ensure!(rowset.rows.iter().all(|r| r.len() == 2));
    ensure!(
        matches!(&rowset.rows[0][1], DbValue::Jsonb(v) if String::from_utf8_lossy(v) == r#"{"s":"hello","n":123,"b":true,"x":null}"#)
    );
    ensure!(
        matches!(&rowset.rows[1][1], DbValue::Jsonb(v) if String::from_utf8_lossy(v) == r#"{"s":"world","n":234,"b":false,"x":null}"#)
    );

    let rowset = uuid_type(&conn)?;
    ensure!(rowset.rows.iter().all(|r| r.len() == 2));
    ensure!(
        matches!(&rowset.rows[0][1], DbValue::Uuid(v) if v == "12345678-1234-1234-1234-123456789abc")
    );
    ensure!(
        matches!(&rowset.rows[1][1], DbValue::Uuid(v) if v == "fedcba98-fedc-fedc-fedc-fedcba987654")
    );

    let rowset = nullable(&conn)?;
    ensure!(rowset.rows.iter().all(|r| r.len() == 1));
    ensure!(matches!(rowset.rows[0][0], DbValue::DbNull));

    let pid1 = format!("{:?}", pg_backend_pid(&conn)?);
    let pid2 = format!("{:?}", pg_backend_pid(&conn)?);
    ensure!(pid1 == pid2);

    Ok(helper::ok_response())
}

fn test_numeric_types(conn: &Connection) -> anyhow::Result<()> {
    const R_SMALLINT: i16 = 13;
    const R_INT2: i16 = 14;
    const R_INT: i32 = 15;
    const R_INT4: i32 = 16;
    const R_BIGINT: i64 = 17;
    const R_INT8: i64 = 18;
    const R_REAL: f32 = 19.2;
    const R_DOUBLE: f64 = 20.3;
    const R_NUMERIC: &str = "123456789.123456789";

    let sql = r#"
        INSERT INTO test_numeric_types(
            id,
            rsmallint,
            rint2,
            rint,
            rint4,
            rbigint,
            rint8,
            rreal,
            rdouble,
            rnumeric
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
        );
    "#;

    conn.execute(
        sql,
        &[
            ParameterValue::Int32(2), // id used only for sorting
            ParameterValue::Int16(R_SMALLINT),
            ParameterValue::Int16(R_INT2),
            ParameterValue::Int32(R_INT),
            ParameterValue::Int32(R_INT4),
            ParameterValue::Int64(R_BIGINT),
            ParameterValue::Int64(R_INT8),
            ParameterValue::Floating32(R_REAL),
            ParameterValue::Floating64(R_DOUBLE),
            ParameterValue::Decimal(R_NUMERIC.to_owned()),
        ],
    )?;

    let sql = r#"
        SELECT
            id,
            rsmallserial,
            rsmallint,
            rint2,
            rserial,
            rint,
            rint4,
            rbigserial,
            rbigint,
            rint8,
            rreal,
            rdouble,
            rnumeric
        FROM test_numeric_types
        ORDER BY id;
    "#;

    let rowset = conn.query(sql, &[])?;

    ensure!(rowset.rows.len() == 2);
    ensure!(rowset.rows.iter().all(|r| r.len() == 13));

    // Spin correctly decodes Postgres values
    ensure!(matches!(rowset.rows[0][1], DbValue::Int16(_))); // rsmallserial
    ensure!(matches!(rowset.rows[0][2], DbValue::Int16(1))); // rsmallint
    ensure!(matches!(rowset.rows[0][3], DbValue::Int16(2))); // rint2
    ensure!(matches!(rowset.rows[0][4], DbValue::Int32(_))); // rserial
    ensure!(matches!(rowset.rows[0][5], DbValue::Int32(3))); // rint
    ensure!(matches!(rowset.rows[0][6], DbValue::Int32(4))); // rint4
    ensure!(matches!(rowset.rows[0][7], DbValue::Int64(_))); // rbigserial
    ensure!(matches!(rowset.rows[0][8], DbValue::Int64(5))); // rbigint
    ensure!(matches!(rowset.rows[0][9], DbValue::Int64(6))); // rint8
    ensure!(matches!(rowset.rows[0][10], DbValue::Floating32(7.8))); // rreal
    ensure!(matches!(rowset.rows[0][11], DbValue::Floating64(9.1))); // rdouble
    ensure!(
        matches!(rowset.rows[0][12], DbValue::Decimal(ref d) if Decimal::from_str_exact(d)? == Decimal::from_i128_with_scale(123456789i128, 9))
    ); // rnumeric

    // Values inserted by Spin round-trip correctly
    // (we can omit the serials here as those are set by the database)
    ensure!(matches!(rowset.rows[1][2], DbValue::Int16(R_SMALLINT))); // rsmallint
    ensure!(matches!(rowset.rows[1][3], DbValue::Int16(R_INT2))); // rint2
    ensure!(matches!(rowset.rows[1][5], DbValue::Int32(R_INT))); // rint
    ensure!(matches!(rowset.rows[1][6], DbValue::Int32(R_INT4))); // rint4
    ensure!(matches!(rowset.rows[1][8], DbValue::Int64(R_BIGINT))); // rbigint
    ensure!(matches!(rowset.rows[1][9], DbValue::Int64(R_INT8))); // rint8
    ensure!(matches!(rowset.rows[1][10], DbValue::Floating32(R_REAL))); // rreal
    ensure!(matches!(rowset.rows[1][11], DbValue::Floating64(R_DOUBLE))); // rdouble
    ensure!(
        matches!(rowset.rows[1][12], DbValue::Decimal(ref d) if Decimal::from_str_exact(d)? == Decimal::from_i128_with_scale(123456789123456789i128, 9))
    ); // rnumeric

    Ok(())
}

fn character_types(conn: &Connection) -> Result<RowSet, PgError> {
    let create_table_sql = r#"
        CREATE TEMPORARY TABLE test_character_types (
            rvarchar varchar(40) NOT NULL,
            rtext text NOT NULL,
            rchar char(10) NOT NULL
         );
    "#;

    conn.execute(create_table_sql, &[])?;

    let insert_sql = r#"
        INSERT INTO test_character_types
            (rvarchar, rtext, rchar)
        VALUES
            ('rvarchar', 'rtext', 'rchar');
    "#;

    conn.execute(insert_sql, &[])?;

    let sql = r#"
        SELECT
            rvarchar, rtext, rchar
        FROM test_character_types;
    "#;

    conn.query(sql, &[])
}

fn date_time_types(conn: &Connection) -> Result<RowSet, PgError> {
    let create_table_sql = r#"
        CREATE TEMPORARY TABLE test_date_time_types (
            index int2,
            rdate date NOT NULL,
            rtime time NOT NULL,
            rtimestamp timestamp NOT NULL
         );
    "#;

    conn.execute(create_table_sql, &[])?;

    // We will use this to test that we correctly decode "known good"
    // Postgres database values. (This validates our decoding logic
    // independently of our encoding logic.)
    let insert_sql_pg_literals = r#"
        INSERT INTO test_date_time_types
            (index, rdate, rtime, rtimestamp)
        VALUES
            (1, date '2525-12-25', time '04:05:06.789', timestamp '1989-11-24 01:02:03');
    "#;

    conn.execute(insert_sql_pg_literals, &[])?;

    // We will use this to test that we correctly encode Spin ParameterValue
    // objects. (In conjunction with knowing that our decode logic is good,
    // this validates our encode logic.)
    let insert_sql_spin_parameters = r#"
        INSERT INTO test_date_time_types
            (index, rdate, rtime, rtimestamp)
        VALUES
            (2, $1, $2, $3);
        "#;

    let date_pv = ParameterValue::Date((2525, 12, 25));
    let time_pv = ParameterValue::Time((14, 15, 16, 17));
    let ts_pv = ParameterValue::Datetime((1989, 11, 24, 1, 2, 3, 4));
    conn.execute(insert_sql_spin_parameters, &[date_pv, time_pv, ts_pv])?;

    let sql = r#"
        SELECT
            index,
            rdate,
            rtime,
            rtimestamp
        FROM test_date_time_types
        ORDER BY index;
    "#;

    conn.query(sql, &[])
}

fn json_types(conn: &Connection) -> Result<RowSet, PgError> {
    let create_table_sql = r#"
        CREATE TEMPORARY TABLE test_json_types (
            index int2,
            j jsonb NOT NULL
         );
    "#;

    conn.execute(create_table_sql, &[])?;

    // We will use this to test that we correctly decode "known good"
    // Postgres database values. (This validates our decoding logic
    // independently of our encoding logic.)
    let insert_sql_pg_literals = r#"
        INSERT INTO test_json_types
            (index, j)
        VALUES
            (1, jsonb('{ "s": "hello", "n": 123, "b": true, "x": null }'));
    "#;

    conn.execute(insert_sql_pg_literals, &[])?;

    // We will use this to test that we correctly encode Spin ParameterValue
    // objects. (In conjunction with knowing that our decode logic is good,
    // this validates our encode logic.)
    let insert_sql_spin_parameters = r#"
        INSERT INTO test_json_types
            (index, j)
        VALUES
            (2, $1);
        "#;

    let jsonb_pv = ParameterValue::Jsonb(
        r#"{ "s": "world", "n": 234, "b": false, "x": null }"#
            .as_bytes()
            .to_vec(),
    );
    conn.execute(insert_sql_spin_parameters, &[jsonb_pv])?;

    let sql = r#"
        SELECT
            index,
            j
        FROM test_json_types
        ORDER BY index;
    "#;

    conn.query(sql, &[])
}

fn uuid_type(conn: &Connection) -> Result<RowSet, PgError> {
    let create_table_sql = r#"
        CREATE TEMPORARY TABLE test_uuid_type (
            index int2,
            u uuid NOT NULL
         );
    "#;

    conn.execute(create_table_sql, &[])?;

    // We will use this to test that we correctly decode "known good"
    // Postgres database values. (This validates our decoding logic
    // independently of our encoding logic.)
    let insert_sql_pg_literals = r#"
        INSERT INTO test_uuid_type
            (index, u)
        VALUES
            (1, uuid('12345678-1234-1234-1234-123456789abc'));
    "#;

    conn.execute(insert_sql_pg_literals, &[])?;

    // We will use this to test that we correctly encode Spin ParameterValue
    // objects. (In conjunction with knowing that our decode logic is good,
    // this validates our encode logic.)
    let insert_sql_spin_parameters = r#"
        INSERT INTO test_uuid_type
            (index, u)
        VALUES
            (2, $1);
        "#;

    let uuid_pv = ParameterValue::Uuid("fedcba98-fedc-fedc-fedc-fedcba987654".to_owned());
    conn.execute(insert_sql_spin_parameters, &[uuid_pv])?;

    let sql = r#"
        SELECT
            index,
            u
        FROM test_uuid
        ORDER BY index;
    "#;

    conn.query(sql, &[])
}

fn nullable(conn: &Connection) -> Result<RowSet, PgError> {
    let create_table_sql = r#"
        CREATE TEMPORARY TABLE test_nullable (
            rvarchar varchar(40)
         );
    "#;

    conn.execute(create_table_sql, &[])?;

    let insert_sql = r#"
        INSERT INTO test_nullable
            (rvarchar)
        VALUES
            ($1);
    "#;

    conn.execute(insert_sql, &[ParameterValue::DbNull])?;

    let sql = r#"
        SELECT
            rvarchar
        FROM test_nullable;
    "#;

    conn.query(sql, &[])
}

fn pg_backend_pid(conn: &Connection) -> Result<DbValue, PgError> {
    let sql = "SELECT pg_backend_pid()";

    let rowset = conn.query(sql, &[])?;

    Ok(rowset.rows[0][0].clone())
}

fn get_header(request: &IncomingRequest, header_key: impl Into<String>) -> anyhow::Result<String> {
    let header_key = header_key.into();
    helper::get_header(request, &header_key).with_context(|| format!("no {} header", header_key))
}
