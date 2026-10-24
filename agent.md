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

### Release verification follow-up

- The first public CI run exposed a missing libxkbcommon-x11 runtime library in
  both Linux runners. Added its development/runtime package and documented the
  minimal Linux runtime dependencies; no application data/parser failure occurred.
- Windows x86_64/ARM64 and macOS ARM64 native build, tests, NumPy oracle and actual
  wheel installation passed in that first run. Remaining/final status is tracked
  separately; failed initial CI is not claimed as a complete release verification.
- Added directional isolates for Arabic formatted indices/numbers/paths. Native
  Unicode and clipboard copies remain unchanged. 31 tests and strict clippy pass.
- Six languages passed 11 real Windows GUI screenshots each; the standard isolated
  sdist-to-wheel build and pip installation passed. A vendored source archive
  resolves all 313 packages offline with an empty dependency cache.

### Final delivery

- Final native CI run 40091597920 passed all six OS/CPU combinations, 31 Rust tests
  and 24 NumPy compatibility cases per platform, plus actual wheel installation.
  Both Linux architectures rendered 66 frames (six languages × 11) from the
  installed portable binaries. Rendered Linux Chinese/Arabic frames were inspected.
- Downloaded all artifacts and independently compared each standalone executable
  with its matching wheel, inspected architectures/minimum OS/runtime imports,
  checked font/crate notices and matched core source files to commit 063512d.
  The final corresponding-source archive resolves all 313 packages offline.
- The final CI Windows wheel was installed and its Chinese GUI rendered 11 frames.
  Twine checks pass for six wheels and the standard source distribution.
- Local large-file upload transport stalled. Added an artifact-based GitHub release
  workflow rather than rebuilding. Release run 40093347966 succeeded and published
  the 0.1.0 prerelease with 15 assets (six standalone archives, six wheels, two source
  archives and SHA256SUMS). All public server SHA-256 digests match local files.
- Existing GitHub credentials authorized the new public repository. pypi/testpypi
  environments are prepared and restricted to main. No PyPI upload was attempted:
  account registration/email verification/2FA and the pending Trusted Publisher
  must be configured by the user. See docs/publishing.md for exact fields.
- No optional MCP was enabled, no private recording was uploaded, and sibling
  projects were left unchanged. The standalone repository is ready for ongoing work.
