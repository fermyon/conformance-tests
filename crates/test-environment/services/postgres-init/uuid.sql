CREATE TABLE test_uuid_type (
    id integer,
    u uuid NOT NULL
);

INSERT INTO test_uuid_type
    (id, u)
VALUES
    (1, uuid('12345678-1234-1234-1234-123456789abc'));
