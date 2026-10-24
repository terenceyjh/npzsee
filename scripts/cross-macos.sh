#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
source "$(dirname "$0")/linux-env.sh"
cd "$task_project_root"
export CARGO_TARGET_DIR="$task_project_root/target/cross"
export MACOSX_DEPLOYMENT_TARGET=13.0
: "${SDKROOT:=$task_project_root/.tools/MacOSX11.3.sdk}"
export SDKROOT
if [ ! -d "$SDKROOT" ]; then
    printf 'Set SDKROOT to a legally obtained macOS SDK before building.\n' >&2
    exit 1
fi
rustup target add aarch64-apple-darwin
cargo zigbuild --release --locked --target aarch64-apple-darwin --bin npzsee
maturin build --release --locked --zig --target aarch64-apple-darwin --out dist
