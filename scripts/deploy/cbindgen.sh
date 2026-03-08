#!/bin/bash
cd "$(dirname "$(readlink -f "$0")")/../.."

if ! command -v cbindgen &> /dev/null; then
  cargo install cbindgen
fi

rm ffi/.temp/jvlib.h
RUSTUP_TOOLCHAIN=nightly \
cbindgen \
  --config cbindgen.toml \
  ffi \
  --output ffi/.temp/jvlib.h \
  --quiet

cd -
