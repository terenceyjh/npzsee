# Release status: NPZSee 0.1.0

Repository: <https://github.com/terenceyjh/npzsee>

- Name: NPZSee; distribution, command and Rust crate: `npzsee`.
- Complete standalone source repository prepared with GPL v3-or-later and
  explicit credit to LiukDiihMieu/npyquick.
- General-purpose interface copy, six interface languages, remembered language
  choice, embedded fonts and docked Table cell/JSON inspection implemented.
- Windows checks: 31 Rust tests, strict clippy and formatting pass.
  Current dependency audit: 312 locked registry packages, no findings.
- Six native platform builds, pip installation checks, standalone archives,
  source archives and checksum manifests are defined in `wheels.yml`.
- Windows installed-wheel GUI checks: all six languages, 11 frames each; long
  cell content, multidimensional navigation, plot and histogram were rendered.
- Standard isolated source-distribution-to-wheel rebuild and pip install pass.
  All 313 source packages resolve offline with an empty Cargo dependency cache.
- The initial CI run passed Windows x86_64/ARM64 and macOS ARM64; both Linux jobs
  exposed a missing X11 keyboard runtime library in the runner setup. The setup
  and runtime documentation were corrected before the final release build.
- Final six-platform CI results and assembled downloads will be linked here when
  the corrected build completes.
- The user reported basic Mac ARM64 operation of the predecessor build. That
  report is not a claim that every interaction in this renamed build was tested.
- PyPI has not been published. The account holder must register/verify their
  account, enable 2FA and add the Trusted Publisher described in publishing.md.
