# NPZSee

**A fast native viewer for multidimensional NumPy arrays.**

[简体中文](README.zh-CN.md) · [Downloads](https://github.com/terenceyjh/npzsee/releases) · [VS Code extension](https://github.com/terenceyjh/NPZSee-vscode) · [Platform notes](docs/cross-platform.md)

Open `.npy` and `.npz` files, navigate dimensions, inspect exact values, and plot
curves and distributions. NPZSee is written in **Rust with egui**. Standalone
downloads need no Python, NumPy, Qt, or Matplotlib.

This repository provides the **standalone desktop edition** for Windows, macOS
and Linux. To view `.npy` and `.npz` files inside your editor, use the companion
**[NPZSee for VS Code](https://github.com/terenceyjh/NPZSee-vscode)** extension.

## Install and run

Download the standalone archive matching your OS and CPU from
[GitHub Releases](https://github.com/terenceyjh/npzsee/releases), then extract it:

- **Windows:** double-click `npzsee.exe`.
- **macOS 13+:** open `NPZSee.app`. Choose `arm64` for Apple Silicon and `x86_64` for Intel.
- **Linux:** run `./npzsee` in a graphical desktop session. A glibc 2.28+ system,
  X11 or Wayland, and working OpenGL drivers are required.

Pass a file path with `npzsee "path/to/arrays.npz"`, or drop a file on the window.
Downloads are currently unsigned release candidates.

### pip

Python 3.10+ can install the matching precompiled wheel. **The package has not
yet been published to PyPI.** For a downloaded wheel:

```sh
python -m pip install ./npzsee-0.1.0-py3-none-<your-platform>.whl
npzsee
npzsee "path/to/arrays.npz"
```

After the first PyPI publication, installation will be `python -m pip install
npzsee`. The wheel installs the same native executable as the standalone
download, with no Python runtime dependencies. Without a matching wheel,
building the source distribution requires Rust and desktop development
libraries. See [publishing](docs/publishing.md).

## Features

- **Array browser:** list and filter NPZ entries immediately; load arrays only
  when selected. Loading and analysis run in a cancellable background worker.
- **Multidimensional navigation:** descend through subarrays to a matrix or
  vector and return with breadcrumbs. C and Fortran memory orders both work.
- **Table first:** inspect exact numeric values, Unicode strings, scalars and
  JSON trees. Click a cell or JSON value to open its **docked detail pane**, read
  the full content and copy it without truncation. Long content does not open
  hover popups.
- **Large tables:** virtual rows and row/column pages keep every value reachable.
- **Plot:** vector, row and two-row/two-column X/Y views; complex components;
  zoom, pan, log axes, original-sample readouts and SVG export.
- **Histogram:** filled distributions, configurable bins, non-finite counts and
  summary statistics. A labelled sampled preview avoids scanning everything;
  exact statistics scan the entire selected slice when requested.
- **Six interface languages:** English, 简体中文, Français, Español, Русский,
  العربية. Change language in the toolbar; the choice is remembered. Arabic
  uses bidirectional layout and native contextual glyph shaping. Fonts are
  bundled under the SIL Open Font License.
- **Read only:** source files are never modified. Object/pickle arrays are not
  executed; store dictionary metadata as Unicode JSON instead.

## Performance

NPY arrays are mapped read only; selected compressed NPZ members are decoded to
temporary mapped files. A bounded cache helps with repeated selections. Plot
overviews use min/max envelopes sized to the viewport, preserving spikes while
drawing thousands of points instead of millions. Zoomed index plots revisit
original samples.

These design choices matter as well as native execution: Rust does not make ZIP
decompression inherently faster than NumPy. Prior local measurements on a
12.5-million-element float64 vector were approximately 240 ms to decode/map,
130 ms to build the envelope and below 1 ms to produce overview geometry. These
are warm-cache component timings, not startup or cross-platform promises.

## Supported data

NPY format versions 1/2/3, common integers, float16/32/64, complex64/128,
booleans, Unicode and byte strings, and either byte order are supported.
Structured fields have text previews. Unsupported fixed-width types show raw
bytes; object arrays require conversion. X/Y overview plots connect samples in
array order; use Table to inspect individual exact values.

## Build and validate

Rust 1.96+ is required. Windows needs MSVC build tools; Linux needs desktop
development libraries; macOS needs Xcode Command Line Tools.

```sh
cargo build --release --locked
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --check
```

The executable is in `target/release/`. For the independent NumPy compatibility
oracle, install NumPy, build with `--features bench`, and run
`python native/validate_numpy.py`. The release workflow builds wheels and
standalone archives for Windows, Linux and macOS, each on x86_64 and ARM64.
Actual checks and limitations are in [release status](docs/release-status.md).

Language strings live in `locales/`. Translations can be improved independently
of the parser. Settings use the normal per-user configuration directory;
`NPZSEE_CONFIG_DIR` overrides that directory. `--language en` overrides startup
language without changing the saved preference.

## Credits and license

NPZSee is derived from and inspired by
[LiukDiihMieu/npyquick](https://github.com/LiukDiihMieu/npyquick). Thank you to the
original project for its NumPy viewing workflow. NPZSee rewrites data loading
and the desktop interface in Rust and adds hierarchical navigation, background
analysis, large-array rendering, localization and native packaging. It is
published under its own name while retaining upstream attribution.

Copyright © 2026 LiukDiihMieu and NPZSee contributors. Licensed under **GNU GPL
version 3 or later**; see [LICENSE](LICENSE), [NOTICE](NOTICE), and the retained
third-party notices in [LICENSES](LICENSES/). Matching source archives including
locked dependency sources accompany release binaries.
