FROM rust:1.30

ADD . /tmp
WORKDIR /tmp
RUN cargo build --release

FROM rust:1.30
WORKDIR /root/
COPY --from=0 /tmp/target/release/object-spam ./
ENTRYPOINT ["./object-spam"]
CMD ["--help"]
