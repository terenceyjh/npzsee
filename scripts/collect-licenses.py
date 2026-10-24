#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Collect dependency notices from the locked Cargo metadata, on any build host."""
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--locked"], cwd=ROOT, encoding="utf-8"))
destination = ROOT / "LICENSES" / "crates"
rows = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package.get("source") is None:
        continue
    source = Path(package["manifest_path"]).parent
    files = []
    for path in source.rglob("*"):
        if not path.is_file():
            continue
        name = path.name.lower()
        if (name.startswith(("license", "licence", "copyright", "notice"))
                or name in ("ofl.txt", "ufl.txt", "unlicense")):
            relative = path.relative_to(source)
            target = destination / f'{package["name"]}-{package["version"]}' / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, target)
            files.append(relative.as_posix())
    rows.append({"name": package["name"], "version": package["version"],
                 "license": package.get("license"), "repository": package.get("repository"),
                 "notice_files": files})
output = ROOT / "LICENSES" / "dependencies.json"
output.write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
(ROOT / "LICENSES" / "README.md").write_text(
    "# Third-party notices\n\n"
    "Generated from Cargo.lock by scripts/collect-licenses.py. The catalogue\n"
    "includes optional and target-specific dependencies, not all of which are\n"
    "linked into each binary. OR expressions offer a choice of license; the\n"
    "GPL-compatible permissive alternative is used. Original copyright and\n"
    "license texts are retained per crate. Bundled Noto font notices are in\n"
    "fonts/. System fonts and platform SDKs are not redistributed.\n",
    encoding="utf-8")
print(f"Collected notices for {len(rows)} locked packages")
