CREATE TABLE test_json_types (
    id integer,
    j jsonb NOT NULL
);

INSERT INTO test_json_types
    (id, j)
VALUES
    (1, jsonb('{ "s": "hello", "n": 123, "b": true, "x": null }'));
