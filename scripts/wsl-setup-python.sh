#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
task_root="$(cd -- "$(dirname -- "$0")/.." && pwd)"
task_venv="$task_root/.tools/packaging-linux"
python3 -m venv --without-pip "$task_venv"
PYTHONPATH=/usr/lib/python3/dist-packages "$task_venv/bin/python3" -m pip install \
    --index-url https://pypi.tuna.tsinghua.edu.cn/simple --ignore-installed pip
"$task_venv/bin/python3" -m pip install --index-url https://pypi.tuna.tsinghua.edu.cn/simple \
    maturin==1.15.0 cargo-zigbuild==0.24.0 ziglang==0.16.0 patchelf numpy
