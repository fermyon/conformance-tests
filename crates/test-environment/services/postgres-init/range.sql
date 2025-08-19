CREATE TABLE test_range_types (
    id integer,
    r4 int4range NOT NULL,
    r8 int8range NOT NULL,
    rnum numrange NOT NULL
);

INSERT INTO test_range_types
    (id, r4, r8, rnum)
VALUES
    (1, '(1, 40]', '[123456789012, 234567890123)', numrange(NULL, 100));
