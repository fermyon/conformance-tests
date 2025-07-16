FROM postgres

ENV POSTGRES_USER=postgres
ENV POSTGRES_PASSWORD=postgres
ENV POSTGRES_DB=spin_dev

COPY ./postgres-init/ /docker-entrypoint-initdb.d/
