#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Render real GUI frames in all six languages, checking the application's report."""
import argparse
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("executable", type=Path)
parser.add_argument("--languages", nargs="+", default=["en", "zh-CN", "fr", "es", "ru", "ar"])
arguments = parser.parse_args()
for language in arguments.languages:
    output = ROOT / "validation-output" / f"gui-{language}"
    output.mkdir(parents=True, exist_ok=True)
    report = output / "smoke.txt"
    error = output / "smoke-error.txt"
    report.unlink(missing_ok=True)
    error.unlink(missing_ok=True)
    subprocess.run([str(arguments.executable.resolve()), str(ROOT / "validation-output/compatibility.npz"),
                    "--language", language, "--ui-smoke", str(output)], timeout=100, check=True)
    assert not error.exists(), error.read_text(encoding="utf-8") if error.exists() else ""
    contents = report.read_text(encoding="utf-8")
    assert "error=None" in contents and "screenshots=11" in contents, contents
    assert len(list(output.glob("ui-*.png"))) == 11
    print(f"{language}: 11 real rendered frames, including the docked long-cell inspector")
