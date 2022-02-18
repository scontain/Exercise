FROM registry.scontain.com:5050/sconecuratedimages/crosscompilers:ubuntu AS builder

COPY . /otpqr

RUN cd /otpqr \
&& scone cargo build --release --target=x86_64-scone-linux-musl \
&& cp /otpqr/target/x86_64-scone-linux-musl/release/otpqr /bin

FROM alpine:latest

COPY --from=0 /bin/otpqr /bin/
CMD /bin/otpqr
