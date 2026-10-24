#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Check the pip-installed distribution and native command, without a display."""
import importlib.metadata
import os
from pathlib import Path
import subprocess
import sysconfig

package = importlib.metadata.distribution("npzsee")
executable = Path(sysconfig.get_path("scripts")) / ("npzsee.exe" if os.name == "nt" else "npzsee")
for option, expected in (("--version", f"npzsee {package.version}"), ("--help", "npzsee"), ("--license", "GPL")):
    result = subprocess.run([str(executable), option], capture_output=True, text=True, encoding="utf-8", check=True)
    assert expected in result.stdout, (option, result.stdout, result.stderr)
files = [str(f).replace("\\", "/") for f in package.files]
assert any(".dist-info/licenses/LICENSE" in f for f in files)
assert any(".dist-info/licenses/NOTICE" in f for f in files)
assert any("LICENSES/crates/" in f for f in files)
assert not package.requires, package.requires
print(f"Installed npzsee {package.version}: CLI, license notices, and zero Python runtime dependencies verified")
