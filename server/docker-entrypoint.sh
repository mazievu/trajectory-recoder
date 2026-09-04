#!/bin/sh
set -eu

# Public CAs are available through the image's normal ca-certificates bundle.
# An operator may additionally mount a private S3/MinIO CA for an internal
# HTTPS endpoint. Never fall back to disabling certificate verification.
if [ -n "${S3_CA_CERT_PATH:-}" ]; then
    if [ ! -f "$S3_CA_CERT_PATH" ]; then
        echo "S3_CA_CERT_PATH does not name a readable certificate file" >&2
        exit 64
    fi

    install -D -m 0644 "$S3_CA_CERT_PATH" /usr/local/share/ca-certificates/trajectory-s3-ca.crt
    update-ca-certificates >/dev/null
fi

exec /usr/local/bin/trajectory-server
