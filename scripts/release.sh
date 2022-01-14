#!/usr/bin/env bash

BASE_PATH="$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )"
cd $BASE_PATH
source $BASE_PATH/source.sh

UPLOAD_URL=$(curl $api/releases/tags/$tag | jq -r '.upload_url')
UPLOAD_URL=${UPLOAD_URL/\{?name,label\}/}
info $UPLOAD_URL

rustup target add "$RUSTTARGET"
cargo build --release --target "$RUSTTARGET"

OUTPUT="target/${RUSTTARGET}/release/idgener"
CHECKSUM=$(sha256sum "${OUTPUT}" | cut -d ' ' -f 1)
FILE_NAME="idgener-${RENAME}"

curl \
  -X POST \
  --data-binary @"${OUTPUT}" \
  -H 'Content-Type: application/octet-stream' \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  "${UPLOAD_URL}?name=${FILE_NAME}"

curl \
  -X POST \
  --data "$CHECKSUM ${FILE_NAME}" \
  -H 'Content-Type: text/plain' \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  "${UPLOAD_URL}?name=${FILE_NAME}.sha256sum"
