# syntax=docker/dockerfile:1.7

FROM scratch

ARG BINARY_PATH=dist/confluence-dl
ARG CHECKSUM_PATH=dist/confluence-dl.sha256

LABEL org.opencontainers.image.title="confluence-dl" \
      org.opencontainers.image.description="CLI utility for exporting Confluence content to Markdown" \
      org.opencontainers.image.vendor="confluence-dl" \
      org.opencontainers.image.licenses="MIT"

COPY ${BINARY_PATH} /confluence-dl
COPY ${CHECKSUM_PATH} /confluence-dl.sha256

ENTRYPOINT ["/confluence-dl"]
