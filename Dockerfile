FROM scratch
COPY target/x86_64-unknown-linux-musl/release/stormstar /stormstar
EXPOSE 8585
ENTRYPOINT ["/stormstar"]
CMD ["serve"]
