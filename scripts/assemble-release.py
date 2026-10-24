#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Assemble all six platform outputs and checksum manifests for publication."""
import hashlib
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]
incoming = ROOT / "incoming"
destinations = {"pypi": ROOT / "publication/pypi", "release": ROOT / "publication/release"}
for directory in destinations.values():
    directory.mkdir(parents=True, exist_ok=True)
for path in incoming.rglob("*"):
    if not path.is_file() or path.name.endswith(".sha256"):
        continue
    if path.suffix == ".whl" or (path.name.startswith("npzsee-") and path.name.endswith(".tar.gz")
                                  and "corresponding-source" not in path.name):
        destination = destinations["pypi"]
    elif path.name.startswith("NPZSee-") or "corresponding-source" in path.name:
        destination = destinations["release"]
    else:
        continue
    target = destination / path.name
    if target.exists():
        assert target.read_bytes() == path.read_bytes(), path.name
    else:
        shutil.copy2(path, target)
wheels = list(destinations["pypi"].glob("*.whl"))
standalone = list(destinations["release"].glob("NPZSee-*"))
assert len(wheels) == 6, [p.name for p in wheels]
assert len(standalone) == 6, [p.name for p in standalone]
assert len(list(destinations["pypi"].glob("*.tar.gz"))) == 1
assert len(list(destinations["release"].glob("*corresponding-source.tar.gz"))) == 1
for directory in destinations.values():
    lines = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n"
             for path in sorted(directory.iterdir()) if path.name != "SHA256SUMS"]
    (directory / "SHA256SUMS").write_text("".join(lines), encoding="utf-8")
print("Six wheels, six standalone archives and matching source archives assembled")
