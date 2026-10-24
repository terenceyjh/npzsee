# NPZSee development record

## 2026-10-24 — independent release repository

The user approved the name NPZSee, package/command `npzsee`, and execution of
`发布前roadmap.md` through the point requiring their account interaction.
The roadmap authorizes a new public GitHub repository and publication preparation.
The desktop application is the scope; sibling npzview, npzview-vscode and the
original npyquick snapshot must remain unchanged.

- Copied complete tracked desktop sources/assets/licenses into this standalone
  directory. Local data, SDKs, tool caches and old distributions were not copied.
- Retained GPL-3.0-or-later, upstream credit and all third-party font/crate notices.
- Generalized product copy from experimental recordings to NumPy arrays.
- Added six UN interface languages and a remembered toolbar language selector.
  Arabic uses Unicode bidi runs and egui 0.36 HarfRust contextual shaping; original
  array text and copy operations retain logical Unicode. Fonts are embedded.
- Table cells and JSON leaves open an integrated, resizable detail panel. Floating
  cell windows and long-content hover popups were removed; full copies are kept.
- Added platform standalone archives generated from the actual wheel executable,
  macOS app bundles, six-platform native CI, dependency-source archives, and
  checksums. PyPI publishing is a separate manually triggered Trusted Publisher
  workflow and does not store account passwords/tokens in the repository.
- Initial Windows validation: 30 Rust tests, strict clippy, formatting and complete
  locale-key/placeholder checks pass. The lockfile has 312 registry packages;
  the current OSV query reports no advisories. Further release results are in
  docs/release-status.md and the linked GitHub build.

Tool installs stay local. Existing local toolchains may be reused with
NPZSEE_TOOLS_ROOT. No optional MCP service was enabled. Do not upload user data,
local tools, account credentials, SDKs, or validation recordings. Actual CI
execution and user-reported Mac predecessor checks must be distinguished from
GUI verification on each physical platform.
