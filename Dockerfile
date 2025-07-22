FROM ubuntu:24.04

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Run `make dist` before this command or build the artifact in CI.
COPY target/debug/malachitebft-eth-app /app/malachitebft-eth-app

RUN chmod +x /app/malachitebft-eth-app

# Copy licenses
#COPY LICENSE ./

EXPOSE 30303 30303/udp 9001 8551 8545 8546
ENTRYPOINT ["/app/malachitebft-eth-app"]
