#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Query public OSV/RustSec records for all versions in Cargo.lock."""
import argparse
import json
from pathlib import Path
import sys
import urllib.request
try:
    import tomllib
except ImportError:
    import tomli as tomllib

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--proxy", help="Optional proxy URL for this request only")
args = parser.parse_args()
lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
packages = [p for p in lock["package"] if p.get("source", "").startswith("registry+")]
body = json.dumps({"queries": [{"package": {"ecosystem": "crates.io", "name": p["name"]},
                               "version": p["version"]} for p in packages]}).encode()
request = urllib.request.Request("https://api.osv.dev/v1/querybatch", data=body,
                                 headers={"Content-Type": "application/json"})
opener = urllib.request.build_opener(urllib.request.ProxyHandler({"https": args.proxy})) if args.proxy else urllib.request.build_opener()
with opener.open(request, timeout=45) as response:
    results = json.load(response)["results"]
assert len(results) == len(packages), "Incomplete advisory response"
findings = [{"package": p["name"], "version": p["version"], "vulns": r["vulns"]}
            for p, r in zip(packages, results) if r.get("vulns")]
destination = root / "validation-output/dependency-audit.json"
destination.parent.mkdir(exist_ok=True)
destination.write_text(json.dumps({"checked_packages": len(packages), "findings": findings}, indent=2), encoding="utf-8")
print(json.dumps({"checked_packages": len(packages), "findings": findings}, indent=2))
sys.exit(bool(findings))
