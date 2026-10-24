#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Inspect wheel machine types, macOS minimums, Windows CRT imports and sdists."""
import hashlib
import json
from pathlib import Path
import re
import struct
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
reports = []
for wheel in sorted((ROOT / "dist").glob("*.whl")):
    with zipfile.ZipFile(wheel) as archive:
        names = archive.namelist()
        binaries = [n for n in names if ".data/scripts/" in n]
        assert len(binaries) == 1, (wheel.name, binaries)
        assert Path(binaries[0]).name in ("npzsee", "npzsee.exe")
        binary = archive.read(binaries[0])
        report = {"file": wheel.name, "bytes": wheel.stat().st_size,
                  "sha256": hashlib.sha256(wheel.read_bytes()).hexdigest()}
        assert any(".dist-info/licenses/NOTICE" in n for n in names)
        assert any(".dist-info/licenses/LICENSES/crates/" in n for n in names)
        assert not any(".tools/" in n or n.endswith(".npz") for n in names)
        if "macosx_" in wheel.name:
            magic, cpu, _, kind, count, _, _, _ = struct.unpack_from("<8I", binary)
            assert magic == 0xFEEDFACF and kind == 2
            arch = {0x100000C: "arm64", 0x1000007: "x86_64"}[cpu]
            assert f"_{arch}.whl" in wheel.name
            tag = re.search(r"macosx_(\d+)_(\d+)_", wheel.name)
            allowed = (int(tag[1]), int(tag[2]), 0)
            offset = 32
            minimum = None
            for _ in range(count):
                command, size = struct.unpack_from("<II", binary, offset)
                if command == 0x32:
                    encoded = struct.unpack_from("<I", binary, offset + 12)[0]
                    minimum = (encoded >> 16, (encoded >> 8) & 255, encoded & 255)
                offset += size
            assert minimum is not None and minimum <= allowed, (wheel.name, minimum, allowed)
            report.update(machine=arch, minimum_macos=minimum)
        elif "win_" in wheel.name:
            pe = struct.unpack_from("<I", binary, 0x3C)[0]
            machine, count = struct.unpack_from("<HH", binary, pe + 4)
            arch = {0x8664: "amd64", 0xAA64: "arm64"}[machine]
            assert f"win_{arch}.whl" in wheel.name
            optional_size = struct.unpack_from("<H", binary, pe + 20)[0]
            optional = pe + 24
            sections = [struct.unpack_from("<IIII", binary, optional + optional_size + i * 40 + 8)
                        for i in range(count)]
            def locate(rva):
                for virtual_size, virtual, raw_size, raw in sections:
                    if virtual <= rva < virtual + max(virtual_size, raw_size):
                        return raw + rva - virtual
                raise ValueError(rva)
            offset = locate(struct.unpack_from("<I", binary, optional + 120)[0])
            libraries = []
            while any(binary[offset:offset + 20]):
                name = locate(struct.unpack_from("<I", binary, offset + 12)[0])
                libraries.append(binary[name:binary.index(b"\0", name)].decode())
                offset += 20
            assert not any(n.lower().startswith(("vcruntime", "msvcp")) for n in libraries), libraries
            report.update(machine=arch, static_vc_runtime=True)
        elif "manylinux_" in wheel.name:
            assert binary[:4] == b"\x7fELF"
            machine = struct.unpack_from("<H", binary, 18)[0]
            arch = {62: "x86_64", 183: "aarch64"}[machine]
            assert f"_{arch}.whl" in wheel.name
            report.update(machine=arch)
        else:
            raise AssertionError(f"Unrecognized platform: {wheel.name}")
        reports.append(report)
for source in (ROOT / "dist").glob("*.tar.gz"):
    with tarfile.open(source) as archive:
        names = [Path(*Path(n).parts[1:]).as_posix() for n in archive.getnames()]
        for required in ("Cargo.toml", "Cargo.lock", "pyproject.toml", "LICENSE", "NOTICE",
                         "native/src/main.rs", "native/src/i18n.rs", "assets/NotoSansCJKsc-Regular.otf",
                         "assets/NotoSans-Regular.ttf", "assets/NotoSansArabic-Regular.ttf",
                         "locales/en.json", "locales/zh-CN.json", "locales/fr.json", "locales/es.json",
                         "locales/ru.json", "locales/ar.json", ".cargo/config.toml"):
            assert required in names, (source.name, required)
        assert not any(n.startswith(("data/", ".tools/", "target/", "validation-output/")) for n in names)
        reports.append({"file": source.name, "bytes": source.stat().st_size,
                        "sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "source_complete": True})
out = ROOT / "validation-output/artifact-check.json"
out.parent.mkdir(exist_ok=True)
out.write_text(json.dumps(reports, indent=2) + "\n", encoding="utf-8")
print(json.dumps(reports, indent=2))
