CREATE TABLE test_date_time_types (
    id integer,
    rdate date NOT NULL,
    rtime time NOT NULL,
    rtimestamp timestamp NOT NULL,
    rinterval interval NOT NULL
);

INSERT INTO test_date_time_types
    (id, rdate, rtime, rtimestamp, rinterval)
VALUES
    (1, date '2525-12-25', time '04:05:06.789', timestamp '1989-11-24 01:02:03', 'P1Y2M3DT4H5M6.7S');
