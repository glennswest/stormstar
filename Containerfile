# StormStar container — stormdbase supervised
FROM registry.gt.lo:5000/stormdbase:latest

# stormstar binary (pre-built for aarch64-musl)
COPY target/aarch64-unknown-linux-musl/release/stormstar /stormstar

# stormd supervisor config
COPY deploy/stormd.toml /etc/stormd/config.toml

VOLUME /data
EXPOSE 8585 80 22

ENTRYPOINT ["/stormd"]
