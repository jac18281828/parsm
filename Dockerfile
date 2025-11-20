FROM ghcr.io/jac18281828/rust:latest

ENV DEBIAN_FRONTEND=noninteractive
RUN sudo apt-get update && \
    sudo apt-get install -y --no-install-recommends \
    python3-venv \
    && \
    sudo apt-get clean && \
    sudo rm -rf /var/lib/apt/lists/* /var/tmp/* /tmp/*

ENV USER=rust
ENV PATH=${PATH}:/home/rust/.cargo/bin:/go/bin
USER rust
