# Cross-platform builds

NPZSee targets **64-bit x86_64 and ARM64** on Windows, Linux and macOS.
32-bit x86 and ARMv7 are not part of this release.

| Platform | Build runner | Runtime baseline |
| --- | --- | --- |
| Windows x86_64 | windows-2022 | Windows 10+ with working desktop/OpenGL |
| Windows ARM64 | windows-11-arm | Windows 11 ARM64 with working desktop/OpenGL |
| Linux x86_64 | ubuntu-22.04 + portable Zig wheel | glibc 2.28+, X11/Wayland, OpenGL |
| Linux ARM64 | ubuntu-24.04-arm + portable Zig wheel | glibc 2.28+, X11/Wayland, OpenGL |
| macOS x86_64 | macos-15-intel | macOS 13+ Intel |
| macOS ARM64 | macos-15 | macOS 13+ Apple Silicon |

See [release status](release-status.md) and the actual linked build results for
what has been executed. Compiling successfully is not a complete interactive
GUI check on every physical machine. The user reported that the predecessor's
macOS ARM64 wheel ran successfully; the newly renamed/localized build requires
its own checks.

Windows statically links the MSVC runtime. Fonts are embedded for all six UI
languages. rfd handles native file dialogs; file paths, drag/drop and clipboard
still depend on the host desktop. Linux requires normal system desktop/driver
libraries even though Python is unnecessary. macOS candidates are not Developer
ID signed or notarized; Apple distribution signing can be added separately.

## Local Windows build

```powershell
.\run-native.ps1 -Build
```

The build script sets a project-local target directory and preserves its caller's
working directory. Alternatively use Cargo normally from the repository root.

## Local WSL toolchain

The scripts can reuse an existing toolchain by setting `NPZSEE_TOOLS_ROOT` to its
`.tools` directory. Otherwise they look in this repository's `.tools` directory.
No tools or SDK are included in source/publication archives.

```sh
bash scripts/wsl-build.sh
bash scripts/linux-validate.sh
bash scripts/build-linux-wheel.sh
```

For macOS cross compilation, legally obtain a macOS SDK and set SDKROOT.
`scripts/cross-macos.sh` targets ARM64 with a minimum OS of 13.0. The six-platform
release workflow uses native runners instead of requiring this optional SDK.

## Standalone downloads

`scripts/package-standalone.py` extracts the exact executable from each validated
wheel and includes GPL, upstream, crate and font notices. macOS downloads contain
`NPZSee.app`; Linux downloads contain an executable `npzsee`; Windows downloads
contain `npzsee.exe`. Extract the whole archive before running.
