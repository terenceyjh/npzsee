#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
source "$(dirname "$0")/linux-env.sh"
cd "$task_project_root"
export CARGO_TARGET_DIR="$task_project_root/target/linux-portable"
maturin build --release --locked --zig --compatibility manylinux_2_28 \
    --target "${1:-x86_64-unknown-linux-gnu}" --out dist
