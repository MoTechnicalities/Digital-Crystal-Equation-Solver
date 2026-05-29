FROM rust:1.88-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
COPY crates/digitalcrystal-engine/Cargo.toml crates/digitalcrystal-engine/Cargo.toml
RUN mkdir -p apps/api/src crates/digitalcrystal-engine/src \
    && printf 'fn main() {}\n' > apps/api/src/main.rs \
    && printf 'pub fn placeholder() {}\n' > crates/digitalcrystal-engine/src/lib.rs \
    && cargo build -p digitalcrystal-api --release

COPY . .
RUN cargo build -p digitalcrystal-api --release

FROM debian:bookworm-slim
WORKDIR /opt/digitalcrystal
COPY --from=builder /app/target/release/digitalcrystal-api /usr/local/bin/digitalcrystal-api
COPY configs/solver.default.toml /opt/digitalcrystal/configs/solver.default.toml
EXPOSE 8080
CMD ["digitalcrystal-api", "--config", "/opt/digitalcrystal/configs/solver.default.toml"]