#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Source this file for the optional project-local WSL toolchain.
task_project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
task_tools_root="${NPZSEE_TOOLS_ROOT:-$task_project_root/.tools}"
if [ -x "$task_tools_root/cargo-linux/bin/cargo" ]; then
    export CARGO_HOME="$task_tools_root/cargo-linux"
    export RUSTUP_HOME="$task_tools_root/rustup-linux"
    export PATH="$CARGO_HOME/bin:$PATH"
fi
export CARGO_TARGET_DIR="$task_project_root/target/linux"
if [ -x "$task_tools_root/packaging-linux/bin/python3" ]; then
    export PATH="$task_tools_root/packaging-linux/bin:$PATH"
    export CARGO_ZIGBUILD_PYTHON_PATH="$task_tools_root/packaging-linux/bin/python3"
    export CARGO_ZIGBUILD_CACHE_DIR="$task_project_root/.cache/zigbuild-linux"
    # Zig atomically renames cache directories; WSL's NTFS mount can reject that.
    # Only disposable build cache lives in /tmp; tools/SDKs remain in the project.
    export ZIG_GLOBAL_CACHE_DIR="${TMPDIR:-/tmp}/npzsee-zig-cache-$UID"
fi
if [ -d "$task_tools_root/sysroot/usr" ]; then
    export PATH="$task_tools_root/sysroot/usr/bin:$PATH"
    export LD_LIBRARY_PATH="$task_tools_root/sysroot/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    export PKG_CONFIG_PATH="$task_tools_root/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
fi
