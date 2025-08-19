CREATE TABLE test_character_types (
    id integer,
    rvarchar varchar(40) NOT NULL,
    rtext text NOT NULL,
    rchar char(10) NOT NULL
);

INSERT INTO test_character_types
    (id, rvarchar, rtext, rchar)
VALUES
    (0, 'rvarchar', 'rtext', 'rchar');
