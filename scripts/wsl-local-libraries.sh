#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Optional Ubuntu build prerequisites, extracted inside the project, no sudo.
set -euo pipefail
task_root="$(cd -- "$(dirname -- "$0")/.." && pwd)"
mkdir -p "$task_root/.tools/sysroot/debs"
cd "$task_root/.tools/sysroot/debs"
apt-get download pkg-config libxkbcommon-dev libxkbcommon0 libegl-dev libegl1
for task_deb in ./*.deb; do dpkg-deb -x "$task_deb" "$task_root/.tools/sysroot"; done
