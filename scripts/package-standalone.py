#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Package the exact portable wheel binary for use without Python or pip."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def package(wheel: Path, output: Path):
    pieces = wheel.name.split("-")
    assert pieces[0] == "npzsee", wheel.name
    version = pieces[1]
    platform = pieces[-1].removesuffix(".whl")
    if platform.startswith("win_"):
        system, architecture = "windows", {"win_amd64": "x86_64", "win_arm64": "arm64"}[platform]
    elif platform.startswith("manylinux_"):
        system, architecture = "linux", "arm64" if platform.endswith("aarch64") else "x86_64"
    elif platform.startswith("macosx_"):
        system, architecture = "macos", "arm64" if platform.endswith("arm64") else "x86_64"
    else:
        raise ValueError(f"Unsupported platform: {platform}")
    name = f"NPZSee-{version}-{system}-{architecture}"
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="npzsee-package-") as temporary:
        directory = Path(temporary) / name
        directory.mkdir()
        executable = directory / ("npzsee.exe" if system == "windows" else "npzsee")
        if system == "macos":
            contents = directory / "NPZSee.app/Contents"
            (contents / "MacOS").mkdir(parents=True)
            executable = contents / "MacOS/npzsee"
            (contents / "Info.plist").write_bytes(plistlib.dumps({
                "CFBundleName": "NPZSee", "CFBundleDisplayName": "NPZSee",
                "CFBundleIdentifier": "io.github.terenceyjh.npzsee",
                "CFBundleExecutable": "npzsee", "CFBundlePackageType": "APPL",
                "CFBundleShortVersionString": version, "CFBundleVersion": version,
                "LSMinimumSystemVersion": "13.0", "NSHighResolutionCapable": True,
                "NSPrincipalClass": "NSApplication",
            }))
        with zipfile.ZipFile(wheel) as archive:
            scripts = [n for n in archive.namelist() if ".data/scripts/" in n and not n.endswith("/")]
            assert len(scripts) == 1, scripts
            executable.write_bytes(archive.read(scripts[0]))
            executable.chmod(0o755)
            for item in archive.namelist():
                marker = ".dist-info/licenses/"
                if marker not in item or item.endswith("/"):
                    continue
                relative = Path(item.split(marker, 1)[1])
                assert not relative.is_absolute() and ".." not in relative.parts
                target = directory / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.read(item))
        command = {"windows": "npzsee.exe", "linux": "./npzsee",
                   "macos": "./NPZSee.app/Contents/MacOS/npzsee"}[system]
        (directory / "START-HERE.txt").write_text(
            f"NPZSee {version} — {system} {architecture}\n\n"
            "Extract the entire archive before running. Python and Rust are not required.\n"
            + ("Double-click NPZSee.app, or run from Terminal:\n" if system == "macos" else "Run from a terminal (Windows: double-click npzsee.exe also works):\n")
            + f'{command}\n{command} "path/to/array.npz"\n\n'
            "Choose one of six languages in the toolbar. Click table cells to open the detail pane.\n"
            "Linux requires a graphical X11/Wayland session and working OpenGL drivers.\n"
            "macOS requires version 13 or later; this candidate is not Developer ID notarized.\n\n"
            "Source, release notes and platform requirements:\nhttps://github.com/terenceyjh/npzsee\n\n"
            "GNU GPL v3 or later. See LICENSE, NOTICE and LICENSES/.\n",
            encoding="utf-8", newline="\n")
        (directory / "BUILD.json").write_text(json.dumps({
            "application": "NPZSee", "version": version, "system": system,
            "architecture": architecture, "wheel": wheel.name,
            "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
        }, indent=2) + "\n", encoding="utf-8")
        if system == "windows":
            destination = Path(shutil.make_archive(str(output / name), "zip", temporary, name))
        else:
            destination = output / f"{name}.tar.gz"
            with tarfile.open(destination, "w:gz", compresslevel=6) as archive:
                archive.add(directory, arcname=name)
        digest = hashlib.sha256(destination.read_bytes()).hexdigest()
        destination.with_name(destination.name + ".sha256").write_text(
            f"{digest}  {destination.name}\n", encoding="utf-8")
        print(destination)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wheels", nargs="+", type=Path)
    parser.add_argument("--out", type=Path, default=ROOT / "release")
    arguments = parser.parse_args()
    for wheel in arguments.wheels:
        package(wheel.resolve(), arguments.out.resolve())
