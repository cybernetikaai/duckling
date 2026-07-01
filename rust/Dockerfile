# Reference container image for the Rust duckling CLI.
# Build context is this `rust/` directory:  docker build -t duckling rust/
#
# Runtime note: jiff resolves IANA time zones from the system tz database at
# runtime, so the image MUST carry `tzdata` (Ubuntu's slim images don't by
# default). That single apt line is the whole reason this file exists — without
# it, TimeZone::get(...) fails and the parser can't coerce into a target zone.

FROM rust:1.85-slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --locked

FROM ubuntu:24.04
# tzdata: the IANA database jiff reads at runtime (kept current via apt on
# rebuild). ca-certificates: harmless, handy if you later add network calls.
RUN apt-get update \
    && apt-get install -y --no-install-recommends tzdata ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/duckling /usr/local/bin/duckling
ENTRYPOINT ["duckling"]
