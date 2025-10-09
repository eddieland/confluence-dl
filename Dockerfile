# syntax=docker/dockerfile:1.7

FROM alpine:3.20 AS certs
RUN apk add --no-cache ca-certificates

FROM scratch

ARG BINARY_PATH=dist/confluence-dl
ARG CHECKSUM_PATH=dist/confluence-dl.sha256

LABEL org.opencontainers.image.title="confluence-dl" \
      org.opencontainers.image.description="CLI utility for exporting Confluence content to Markdown" \
      org.opencontainers.image.vendor="confluence-dl" \
      org.opencontainers.image.licenses="MIT"

COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

COPY ${BINARY_PATH} /confluence-dl
COPY ${CHECKSUM_PATH} /confluence-dl.sha256

ENTRYPOINT ["/confluence-dl"]
