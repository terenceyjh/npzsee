#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Verify downloaded build artifacts and prepare one version's release assets."""
import hashlib
import os
from pathlib import Path
import re
import shutil

ROOT = Path(__file__).resolve().parents[1]
incoming = ROOT / "incoming"
output = ROOT / "publication/github"
tag = os.environ["RELEASE_TAG"]
assert re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[a-z0-9.-]+)?", tag), "Invalid release tag"
assert (ROOT / f"docs/releases/{tag}.md").is_file(), "Missing release notes"
wheels = sorted((incoming / "pypi").glob("*.whl"))
assert len(wheels) == 6
assert all(wheel.name.startswith(f"npzsee-{tag}-") for wheel in wheels)
archives = [path for path in (incoming / "release").iterdir()
            if path.name.startswith(f"NPZSee-{tag}-") and path.name.endswith((".zip", ".tar.gz"))]
assert len(archives) == 6
for line in (incoming / "release/SHA256SUMS").read_text().splitlines():
    digest, name = line.split("  ", 1)
    path = incoming / "release" / name
    assert path.resolve().is_relative_to((incoming / "release").resolve())
    assert hashlib.sha256(path.read_bytes()).hexdigest() == digest, name
sources = [incoming / "pypi" / f"npzsee-{tag}.tar.gz",
           incoming / "release" / f"npzsee-{tag}-corresponding-source.tar.gz"]
assert all(path.is_file() for path in sources)
output.mkdir(parents=True, exist_ok=True)
assets = sorted([*wheels, *archives, *sources])
assert len(assets) == 14
for path in assets:
    shutil.copy2(path, output / path.name)
(output / "SHA256SUMS").write_text("".join(
    f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n" for path in assets), encoding="utf-8")
print("Prepared 14 validated release files and a combined checksum manifest")
