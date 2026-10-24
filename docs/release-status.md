# Release status: NPZSee 0.1.0

- Public repository: <https://github.com/terenceyjh/npzsee>
- Release downloads: <https://github.com/terenceyjh/npzsee/releases/tag/0.1.0>
- Successful release build: <https://github.com/terenceyjh/npzsee/actions/runs/40091597920>
- Validated source commit: `063512d22c3a63a177fa536c8300bbf4ee6b8b91`.

## Delivered

NPZSee is the display name; `npzsee` is the package, command, Rust crate and
repository name. The complete independent repository retains GPL v3-or-later,
upstream credit and third-party notices. The previous desktop/extension and
original upstream folders remain separate.

General-purpose array-viewer text, six UN interface languages, remembered toolbar
language selection, embedded fonts and Arabic bidi/contextual shaping are included.
Clicking table cells or JSON values opens a resizable docked detail panel with
full-copy support; long-content hover windows were removed.

| Platform | Native build, 31 Rust tests, 24 NumPy cases, pip/CLI checks | Rendered GUI checks |
| --- | --- | --- |
| Windows x86_64 | Pass | Six languages locally; final installed CI wheel checked again |
| Windows ARM64 | Pass | Headless interface tests; physical desktop use remains to be exercised |
| Linux x86_64 | Pass | Six languages, 11 frames each, from the installed portable wheel |
| Linux ARM64 | Pass | Six languages, 11 frames each, from the installed portable wheel |
| macOS x86_64 | Pass | Headless interface tests; interactive dialogs/clipboard need physical use |
| macOS ARM64 | Pass | Headless interface tests; user reported basic operation of predecessor |

Six standalone archives, six wheels, a standard source distribution, a matching
source archive with locked Rust dependency sources, and a combined SHA256SUMS are
provided. Each standalone executable was byte-compared with its matching wheel.
PE/ELF/Mach-O machine types, Windows static VC runtime imports, macOS 13.0 minimum,
font/crate notices and source completeness were checked.

The final corresponding source archive matches the validated application's source
files exactly and resolves all 313 packages offline with an empty Cargo dependency
cache. A standard isolated sdist-to-wheel rebuild passed locally. The supplied
large recording's values and exact histogram agree with NumPy; no private data
was committed or included in releases. Current OSV audit: 312 registry packages,
no findings.

The initial CI run identified missing libxkbcommon-x11 in both Linux runners.
The setup/runtime documentation were corrected and the complete final run passed.

## Account-holder step remaining

**PyPI has not been published.** The owner must register/verify their PyPI account,
enable 2FA and add the pending Trusted Publisher in [publishing.md](publishing.md).
GitHub environments `pypi` and `testpypi` already exist and allow the main branch.
The manual publication workflow accepts the successful build ID **40091597920**
and publishes those exact prebuilt files after account configuration.

Current binaries are unsigned release candidates. See cross-platform.md for
runtime libraries and system minimums. Native build/installation and automated
rendering do not replace every interaction test on each physical device.
