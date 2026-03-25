# build stage
FROM rust:latest AS cargo-build

WORKDIR /usr/src/verden
COPY . .

ARG DATABASE_URL="postgres://user:password@localhost:5432/verden"
ARG IPA_CA_URL="https://ipa.lab.students.cs.unibo.it/ipa/config/ca.crt"

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && \
	curl -fsSL --insecure "$IPA_CA_URL" -o /usr/local/share/ca-certificates/freeipa.crt && \
	update-ca-certificates && \
	rm -rf /var/lib/apt/lists/*

RUN cargo install --path . && cargo install sqlx-cli
EXPOSE 9090

CMD ["verden"]
