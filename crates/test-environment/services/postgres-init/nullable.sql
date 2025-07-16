CREATE TABLE test_nullable (
    id integer,
    rvarchar varchar(40)
);

INSERT INTO test_nullable
    (id, rvarchar)
VALUES
    (1, NULL);
