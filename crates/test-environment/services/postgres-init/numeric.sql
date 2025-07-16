CREATE TABLE test_numeric_types (
    id integer,
    rsmallserial smallserial NOT NULL,
    rsmallint smallint NOT NULL,
    rint2 int2 NOT NULL,
    rserial serial NOT NULL,
    rint int NOT NULL,
    rint4 int4 NOT NULL,
    rbigserial bigserial NOT NULL,
    rbigint bigint NOT NULL,
    rint8 int8 NOT NULL,
    rreal real NOT NULL,
    rdouble double precision NOT NULL,
    rnumeric numeric NOT NULL
);

INSERT INTO test_numeric_types
    (id, rsmallint, rint2, rint, rint4, rbigint, rint8, rreal, rdouble, rnumeric)
VALUES
    (0, 1, 2, 3, 4, 5, 6, 7.8, 9.1, 0.123456789);
