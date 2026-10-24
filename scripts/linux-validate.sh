#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
source "$(dirname "$0")/linux-env.sh"
cd "$task_project_root"
cargo test --locked --all-features
cargo build --release --locked --all-features
"$CARGO_TARGET_DIR/release/npzsee" --version
export NPZSEE_BENCH="$CARGO_TARGET_DIR/release/npzsee-bench"
python3 native/validate_numpy.py
mkdir -p validation-output/linux-gui
export LIBGL_ALWAYS_SOFTWARE=1
timeout 90s env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1440x900x24' "$CARGO_TARGET_DIR/release/npzsee" \
    validation-output/compatibility.npz --ui-smoke validation-output/linux-gui
test -f validation-output/linux-gui/smoke.txt
test ! -f validation-output/linux-gui/smoke-error.txt
cat validation-output/linux-gui/smoke.txt
