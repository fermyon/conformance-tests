CREATE TABLE test_array_types (
    id integer,
    i4arr int4[] NOT NULL,
    i8arr int8[] NOT NULL,
    numarr numeric[] NOT NULL,
    strarr text[] NOT NULL
);

INSERT INTO test_array_types
    (id, i4arr, i8arr, numarr, strarr)
VALUES
    (1, '{1,2,3}', '{101, 102, 103, 104}', '{1.234, 2.345}', '{"hello", "mum"}');
