#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
source "$(dirname "$0")/linux-env.sh"
cd "$task_project_root"
cargo build --release --locked --all-features
