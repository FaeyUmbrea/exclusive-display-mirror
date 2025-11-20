# exclusive-display-mirror

[![CI](https://img.shields.io/github/actions/workflow/status/OWNER/REPO/ci.yml?branch=main&style=flat-square)](https://github.com/OWNER/REPO/actions)
[![Release](https://img.shields.io/github/v/release/OWNER/REPO?style=flat-square)](https://github.com/OWNER/REPO/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Languages](https://img.shields.io/github/languages/top/OWNER/REPO?style=flat-square)](https://github.com/OWNER/REPO)

A toolkit for mirroring content to a specialized display on Windows. The project includes a web UI (Svelte + Vite), a Tauri desktop wrapper, and native helper components for capturing or presenting output to dedicated displays.

Important: this project requires specialized display hardware or OS features to function correctly on Windows — see the "Windows specialized display requirements" section below.

Table of contents
- Features
- Requirements
- Quick start
- Development
  - Web UI (Vite)
  - Tauri application (dev)
- Production build & release
- Windows specialized display requirements (critical)
- Project layout
- How build-time assets are handled (`src-tauri/build.rs`)
- Troubleshooting
- Contributing
- License


Features
- Web UI built with Svelte and Vite.
- Native components / helper projects for specialized display workflows (see `display_helper/` and `src-tauri/`).
- Tauri wrapper for cross-platform desktop integration (this project is primarily used on Windows with specialized hardware).


Requirements
- Rust (stable). Recommended: rustup-managed stable toolchain, at least 1.70.0. Install via: https://rustup.rs
- Node.js: Recommended LTS 18.x or later. https://nodejs.org
- Yarn: Yarn classic (>= 1.22) or Yarn 3+ is fine. https://yarnpkg.com
- Windows (for specialized display support). Some parts of the project are Windows-only (native helpers, capture tools).

Note about admin privileges: building or installing native helpers/drivers may require Administrator rights on Windows.


Quick start (clone + install)

1. Clone the repo:

```bash
# replace OWNER/REPO with the repository owner and name
git clone https://github.com/OWNER/REPO.git
cd REPO
```

2. Install JavaScript deps with Yarn:

```bash
yarn install
```

3. Ensure you have Rust toolchain installed (use rustup):

```bash
rustup install stable
rustup default stable
```


Development

Web UI (Vite)

- Start the dev server (hot reload):

```bash
# runs the vite dev script from package.json
yarn dev
```

- Build only the web assets:

```bash
yarn build
```

Tauri application (local development)

You can run the Tauri application during development. There are two common approaches.

- Using the npm/yarn script (recommended when using the JS toolchain):

```bash
# starts the Tauri dev flow via the tauri CLI
yarn tauri dev
```

- Or using the Rust cargo wrapper directly:

```bash
cargo tauri dev
```

Notes:
- Make sure your Rust toolchain and the `tauri-cli` is installed. You can install the `tauri-cli` with:

```bash
cargo install tauri-cli
```

- On Windows you may need to run a terminal as Administrator if native components require elevated privileges during development.


Production build & release

1. Build the web assets:

```bash
yarn build
```

2. Build the Tauri native bundle (this will run the Rust build and package the application):

```bash
# using yarn script
yarn tauri build

# OR using cargo
cargo tauri build
```

Artifacts
- `build/` — the web build output (the repo already contains a `build/` folder produced by the static build step).
- `src-tauri/target/` or `target/` — Rust build artifacts and packaged Tauri bundles. See the `tauri` output from the build command for final installer paths.


Windows specialized display requirements (IMPORTANT)

This project is designed to target dedicated/specialized displays on Windows (for example kiosk panels, signage displays, capture-card-based workflows). The project will only work if one of the following is true for the target machine:

1) You have a specialized display available (such as a kiosk display) that Windows recognizes as a specialized display. These displays sometimes expose flags that allow the OS to treat them specially.

2) You are running Windows Enterprise and have access to the "Remove display from desktop" (or equivalent) option that allows the system to dedicate a display without it being part of the regular desktop.

3) You have an Elgato (or compatible) capture card and you have installed a modified EDID on the capture device that sets the specialized-display flag. In other words: capture-card + modified EDID that signals a specialized display to the OS.

Why this matters:
- Windows will not always present a display as a dedicated output suitable for 'exclusive' mirroring. The OS and drivers may mirror or extend by default. The three options above are ways to ensure a display can be treated as a dedicated target for the application's output.

Practical notes and options
- `display_helper/` — native helper project included in the repo; check its docs for installation/debugging of native drivers or services. It may contain Windows-native code and Visual Studio project files.
- `src-tauri/resources/obs` — resources used by the Tauri/Rust side to support OBS-based workflows; these are copied into the final target directory during the Rust build (see below).
- `win-capture/` — contains packages and metadata related to Windows capture approaches; useful if you plan to drive capture-card workflows.


How build-time assets are handled (`src-tauri/build.rs`)

During the Rust build the file `src-tauri/build.rs` runs. Its current behavior:
- It computes the `src-tauri` directory based on `CARGO_MANIFEST_DIR` and looks for `src-tauri/resources/obs`.
- If that resources directory exists, the build script copies its complete contents into the Cargo target directory for the current profile (for example `target/debug/` or `target/release/`).
- The intent is to ensure OBS-related assets are present next to the compiled binary so they can be packaged or loaded at runtime.

If you need to change this behavior:
- Edit `src-tauri/build.rs` to point at a different resource path or to include/exclude other directories.

Required OBS runtime resources

This project depends on prebuilt OBS runtime files that are not checked in to the repository. They must be present at `src-tauri/resources/obs` prior to building. The project expects the latest `obs-build.7z` release from the libobs-rs builds repository.

To make this easy we've included a helper script at `tools/download_obs.ps1` which:
- Downloads the latest (or specified) `obs-build*.7z` release from https://github.com/libobs-rs/libobs-builds/releases
- Extracts its contents to `src-tauri/resources/obs`

Usage (from repository root):

```powershell
# download latest release and extract (no overwrite)
pwsh tools\download_obs.ps1

# download a specific version (tag name) and overwrite existing files
pwsh tools\download_obs.ps1 -Version 1.2.3 -Force
```

Notes:
- The script looks for `7z.exe` (7-Zip). If not found it attempts to install 7-Zip via Chocolatey (requires admin rights).
- The helper is also used in CI: the Windows release workflow will download and extract the OBS build to ensure builds do not fail due to missing resources.
- If you prefer to manage these resources manually, place the extracted contents directly under `src-tauri/resources/obs`.


Project layout (high level)
- `src/` — Svelte web UI source.
- `src-tauri/` — Tauri/Rust sources, and `build.rs` (copies build-time assets).
- `display_helper/` — native helper (Windows) with C++/Visual Studio/CMake artifacts.
- `win-capture/`, `resources/`, `rtmp-services/` — additional tools and assets.


Troubleshooting

Common problems
- "tauri dev" or "cargo tauri dev" fails to compile:
  - Ensure Rust toolchain is installed and up to date.
  - Run `rustup show` to confirm the default toolchain.
  - If errors reference missing libraries, install Windows build tools (Visual Studio Build Tools).

- Resources not found at runtime:
  - Confirm `src-tauri/resources/obs` exists. `src-tauri/build.rs` will print a cargo warning if it's missing and skip copying.
  - Check the target directory for the current profile: e.g. `target/debug/` or `target/release/`.

- Specialized display not available or not seen by Windows:
  - Confirm device drivers are installed and the display reports the expected EDID flags.
  - For capture-card workflows, ensure your capture device supports loading a custom EDID and that the EDID contains the specialized-display bit/flag.
  - If using Windows Enterprise features, verify policy and UI options are available on your build.

Collecting logs
- Tauri logs: run `cargo tauri dev` and observe the terminal output.
- Rust build errors: copy the full compiler output when filing issues.


Commands reference (cheat-sheet)

```bash
# Install deps
yarn install

# Dev web UI
yarn dev

# Dev Tauri
yarn tauri dev
# or
cargo tauri dev

# Build web assets
yarn build

# Build Tauri bundle
yarn tauri build
# or
cargo tauri build
```


Contributing
- Fork the repository, create a feature branch, add tests, and open a Pull Request.
- Please follow the existing code style and make small, focused PRs.
- If changes affect native code, document platform-specific build steps.


License
- This project is licensed under the MIT License (see `LICENSE`).

Acknowledgements
- Built with Svelte, Vite, Tauri and other OSS components.


Contact / Support
- Open an issue in the repository and include: OS version, Rust and Node versions, reproduction steps, and any relevant logs.


Replace owner/repo
- Before publishing the README, replace the `OWNER/REPO` placeholders in badge URLs and the clone URL with your real GitHub owner and repo name.




