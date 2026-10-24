#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
source "$(dirname "$0")/linux-env.sh"
cd "$task_project_root"
python3 -m pip install --force-reinstall --no-index --find-links dist npzsee
python3 -m pip check
python3 scripts/check-installed.py
mkdir -p validation-output/linux-wheel-gui
export LIBGL_ALWAYS_SOFTWARE=1
timeout 90s env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1440x900x24' \
    npzsee validation-output/compatibility.npz --ui-smoke validation-output/linux-wheel-gui
test -f validation-output/linux-wheel-gui/smoke.txt
test ! -f validation-output/linux-wheel-gui/smoke-error.txt
cat validation-output/linux-wheel-gui/smoke.txt
