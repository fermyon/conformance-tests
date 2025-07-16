CREATE TABLE test_blob_types (
    id integer,
    blob bytea NOT NULL
);

INSERT INTO test_blob_types
    (id, blob)
VALUES
    (1, 'abcdef');
