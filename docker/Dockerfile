FROM postgres:latest

RUN apt-get update && \
    apt-get install -y curl wget && \ 
    rm -rf /var/lib/apt/lists/*

EXPOSE 5432