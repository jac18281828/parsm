FROM jac18281828/rust:latest

ENV USER=rust
ENV PATH=${PATH}:/home/rust/.cargo/bin:/go/bin
USER rust
