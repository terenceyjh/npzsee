#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Build a matching source archive, including vendored locked Rust dependencies."""
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / ".tools"
TOOLS.mkdir(exist_ok=True)
maturin = shutil.which("maturin") or str(TOOLS / "packaging/Scripts/maturin.exe")
subprocess.run([maturin, "sdist", "--out", "dist"], cwd=ROOT, check=True)
vendor = TOOLS / "vendor"
result = subprocess.run(["cargo", "vendor", "--locked", "--versioned-dirs", str(vendor)],
                        cwd=ROOT, check=True, capture_output=True, text=True)
source = max((ROOT / "dist").glob("npzsee-*.tar.gz"), key=lambda p: p.stat().st_mtime)
with tempfile.TemporaryDirectory(prefix="source-bundle-", dir=TOOLS) as tmp:
    temporary = Path(tmp).resolve()
    assert temporary.is_relative_to(TOOLS.resolve())
    with tarfile.open(source) as archive:
        for member in archive.getmembers():
            assert (temporary / member.name).resolve().is_relative_to(temporary)
            assert not member.issym() and not member.islnk()
        archive.extractall(temporary)
    project = next(temporary.iterdir())
    config = re.sub(r'directory = ".*"', 'directory = "vendor"', result.stdout)
    (project / ".cargo").mkdir(exist_ok=True)
    config_path = project / ".cargo/config.toml"
    existing = config_path.read_text(encoding="utf-8") if config_path.exists() else ""
    config_path.write_text(existing + "\n" + config, encoding="utf-8")
    output = ROOT / "release"
    output.mkdir(exist_ok=True)
    destination = output / f"{project.name}-corresponding-source.tar.gz"
    with tarfile.open(destination, "w:gz", compresslevel=6) as archive:
        archive.add(project, arcname=project.name)
        archive.add(vendor, arcname=f"{project.name}/vendor")
    checksum = hashlib.sha256(destination.read_bytes()).hexdigest()
    destination.with_suffix(destination.suffix + ".sha256").write_text(
        f"{checksum}  {destination.name}\n", encoding="utf-8")
    print(destination)
