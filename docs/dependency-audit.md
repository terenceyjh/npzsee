# Dependency advisory check — 2026-10-24

Queried the public OSV API (including RustSec records) for every registry package
version in Cargo.lock. Initial zip 2.4.2 matched RUSTSEC-2026-0367, 0368 and 0369.
Upgraded the dependency to zip 9.0.5 and adapted the fallible filename API.

Those advisories concern archive extraction APIs that this application does not
call; array payloads are copied into anonymous files. The dependency was still
updated before preparing release candidates. See the primary advisories:
[0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html),
[0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html),
[0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html).

The updated lockfile check covered **310 packages and returned no findings**.
This is a point-in-time advisory lookup, not a proof that the application or all
dependencies are free of vulnerabilities. Repeat immediately before publication:

```text
python scripts/audit-dependencies.py
```

An optional `--proxy URL` applies only to that request. Detailed local output is
written to validation-output/dependency-audit.json and is not committed.
