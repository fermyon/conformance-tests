use anyhow::{ensure, Context as _};
use helper::bindings::{
    spin::postgres4_0_0::postgres::{Connection, DbValue, Error as PgError, ParameterValue},
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
    test_character_types(&conn)?;
    test_date_time_types(&conn)?;
    test_json_types(&conn)?;
    test_uuid_type(&conn)?;
    test_nullable(&conn)?;
    test_call_pg_fn(&conn)?;

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

fn test_character_types(conn: &Connection) -> anyhow::Result<()> {
    let insert_sql = r#"
        INSERT INTO test_character_types
            (id, rvarchar, rtext, rchar)
        VALUES
            ($1, $2, $3, $4);
    "#;

    conn.execute(
        insert_sql,
        &[
            ParameterValue::Int32(2),
            ParameterValue::Str("varchar2".to_owned()),
            ParameterValue::Str("text2".to_owned()),
            ParameterValue::Str("char2".to_owned()),
        ],
    )?;

    let sql = r#"
        SELECT
            id, rvarchar, rtext, rchar
        FROM test_character_types
        ORDER BY id;
    "#;

    let rowset = conn.query(sql, &[])?;

    ensure!(rowset.rows.len() == 2);
    ensure!(rowset.rows.iter().all(|r| r.len() == 4));

    // The `rchar` column has a fixed width of 10. This function gives us what
    // the padded result we expect to get from Postgres .
    fn pad10(s: &str) -> String {
        format!("{s: <10}")
    }

    // Spin correctly decodes Postgres values
    ensure!(matches!(rowset.rows[0][1], DbValue::Str(ref s) if s == "rvarchar"));
    ensure!(matches!(rowset.rows[0][2], DbValue::Str(ref s) if s == "rtext"));
    ensure!(matches!(rowset.rows[0][3], DbValue::Str(ref s) if s == &pad10("rchar")));

    // Values inserted by Spin round-trip correctly
    ensure!(matches!(rowset.rows[1][1], DbValue::Str(ref s) if s == "varchar2"));
    ensure!(matches!(rowset.rows[1][2], DbValue::Str(ref s) if s == "text2"));
    ensure!(matches!(rowset.rows[1][3], DbValue::Str(ref s) if s == &pad10("char2")));

    Ok(())
}

fn test_date_time_types(conn: &Connection) -> anyhow::Result<()> {
    let date = (1901, 1, 2);
    let time = (14, 15, 16, 17_000_000);
    let ts = (1999, 7, 12, 4, 3, 2, 1_000_000);
    let interval = helper::bindings::spin::postgres4_0_0::postgres::Interval {
        micros: 123456,
        days: 10,
        months: 200,
    };

    let date_pv = ParameterValue::Date(date);
    let time_pv = ParameterValue::Time(time);
    let ts_pv = ParameterValue::Datetime(ts);
    let interval_pv = ParameterValue::Interval(interval);

    let sql = r#"
        INSERT INTO test_date_time_types
            (id, rdate, rtime, rtimestamp, rinterval)
        VALUES
            (2, $1, $2, $3, $4);
        "#;

    conn.execute(sql, &[date_pv, time_pv, ts_pv, interval_pv])?;

    let sql = r#"
        SELECT
            id,
            rdate,
            rtime,
            rtimestamp,
            rinterval
        FROM test_date_time_types
        ORDER BY id;
    "#;

    let rowset = conn.query(sql, &[])?;

    eprintln!("{rowset:?}");

    ensure!(rowset.rows.len() == 2);
    ensure!(rowset.rows.iter().all(|r| r.len() == 5));

    // Spin correctly decodes Postgres values
    ensure!(
        matches!(rowset.rows[0][1], DbValue::Date((y, m, d)) if y == 2525 && m == 12 && d == 25)
    );
    ensure!(
        matches!(rowset.rows[0][2], DbValue::Time((h, m, s, ns)) if h == 4 && m == 5 && s == 6 && ns == 789_000_000)
    );
    ensure!(
        matches!(rowset.rows[0][3], DbValue::Datetime((y, mo, d, h, min, s, ns)) if y == 1989 && mo == 11 && d == 24 && h == 1 && min == 2 && s == 3 && ns == 0)
    );
    ensure!(
        matches!(rowset.rows[0][4], DbValue::Interval(iv) if iv.months == 14 && iv.days == 3 && iv.micros == 245 * 60 * 1000000 + 6700000 /* 4h 5min 6.7s */)
    );

    // Values inserted by Spin round-trip correctly
    ensure!(matches!(rowset.rows[1][1], DbValue::Date(tuple) if tuple == date));
    ensure!(matches!(rowset.rows[1][2], DbValue::Time(tuple) if tuple == time));
    ensure!(matches!(rowset.rows[1][3], DbValue::Datetime(tuple) if tuple == ts));
    ensure!(
        matches!(rowset.rows[1][4], DbValue::Interval(i) if i.months == interval.months && i.days == interval.days && i.micros == interval.micros)
    );

    Ok(())
}

fn test_json_types(conn: &Connection) -> anyhow::Result<()> {
    let sql = r#"
        INSERT INTO test_json_types
            (id, j)
        VALUES
            (2, $1);
        "#;

    let json_text = r#"{ "s": "world", "n": 234, "b": false, "x": null }"#;
    let jsonb_pv = ParameterValue::Jsonb(json_text.as_bytes().to_vec());
    conn.execute(sql, &[jsonb_pv])?;

    let sql = r#"
        SELECT
            id,
            j
        FROM test_json_types
        ORDER BY id;
    "#;

    let rowset = conn.query(sql, &[])?;

    ensure!(rowset.rows.len() == 2);
    ensure!(rowset.rows.iter().all(|r| r.len() == 2));

    fn json_eq(expected: &str, actual: &[u8]) -> bool {
        let j_expected: serde_json::Value = serde_json::from_str(expected).unwrap();
        let j_actual: serde_json::Value = serde_json::from_slice(actual).unwrap();
        j_expected == j_actual
    }

    ensure!(
        matches!(rowset.rows[0][1], DbValue::Jsonb(ref v) if json_eq(r#"{"s":"hello","n":123,"b":true,"x":null}"#, v))
    );
    ensure!(matches!(rowset.rows[1][1], DbValue::Jsonb(ref v) if json_eq(json_text, v)));

    Ok(())
}

fn test_uuid_type(conn: &Connection) -> anyhow::Result<()> {
    let sql = r#"
        INSERT INTO test_uuid_type
            (id, u)
        VALUES
            (2, $1);
        "#;

    let uuid_pv = ParameterValue::Uuid("fedcba98-fedc-fedc-fedc-fedcba987654".to_owned());
    conn.execute(sql, &[uuid_pv])?;

    let sql = r#"
        SELECT
            id,
            u
        FROM test_uuid_type
        ORDER BY id;
    "#;

    let rowset = conn.query(sql, &[])?;

    ensure!(rowset.rows.len() == 2);
    ensure!(rowset.rows.iter().all(|r| r.len() == 2));

    ensure!(
        matches!(&rowset.rows[0][1], DbValue::Uuid(v) if v == "12345678-1234-1234-1234-123456789abc")
    );
    ensure!(
        matches!(&rowset.rows[1][1], DbValue::Uuid(v) if v == "fedcba98-fedc-fedc-fedc-fedcba987654")
    );

    Ok(())
}

fn test_nullable(conn: &Connection) -> anyhow::Result<()> {
    let insert_sql = r#"
        INSERT INTO test_nullable
            (id, rvarchar)
        VALUES
            (2, $1);
    "#;

    conn.execute(insert_sql, &[ParameterValue::DbNull])?;

    let sql = r#"
        SELECT
            id, rvarchar
        FROM test_nullable
        ORDER BY id;
    "#;

    let rowset = conn.query(sql, &[])?;

    ensure!(rowset.rows.iter().all(|r| r.len() == 2));

    ensure!(matches!(rowset.rows[0][1], DbValue::DbNull));
    ensure!(matches!(rowset.rows[1][1], DbValue::DbNull));

    Ok(())
}

fn test_call_pg_fn(conn: &Connection) -> anyhow::Result<()> {
    let pid1 = format!("{:?}", pg_backend_pid(conn)?);
    let pid2 = format!("{:?}", pg_backend_pid(conn)?);
    ensure!(pid1 == pid2);
    Ok(())
}

#[allow(clippy::result_large_err)]
fn pg_backend_pid(conn: &Connection) -> Result<DbValue, PgError> {
    let sql = "SELECT pg_backend_pid()";

    let rowset = conn.query(sql, &[])?;

    Ok(rowset.rows[0][0].clone())
}

fn get_header(request: &IncomingRequest, header_key: impl Into<String>) -> anyhow::Result<String> {
    let header_key = header_key.into();
    helper::get_header(request, &header_key).with_context(|| format!("no {} header", header_key))
}
