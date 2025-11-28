FROM postgres:latest

COPY db-dump.tar.gz /tmp/db-dump.tar.gz
RUN mkdir /dump
RUN tar -xzf /tmp/db-dump.tar.gz -C /dump --strip-components=1
RUN rm /tmp/db-dump.tar.gz
RUN mv /dump/data /data
RUN rm -r /dump
RUN chown -R postgres:postgres /data

COPY schema.sql /docker-entrypoint-initdb.d/00-schema.sql
COPY import.sql /docker-entrypoint-initdb.d/01-import.sql

RUN chown -R postgres:postgres /docker-entrypoint-initdb.d
RUN chmod -R u+rwX /docker-entrypoint-initdb.d

ENV POSTGRES_DB=crates_io_db
ENV POSTGRES_USER=crates
ENV POSTGRES_PASSWORD=crates

EXPOSE 5432