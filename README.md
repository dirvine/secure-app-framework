# secure-app-framework

**A reference implementation of a secure, auditable, cross‑platform app** using **WASM + native broker** (Option A) with an ultra‑modern, native‑feeling UI and a rigorous, reproducible software supply chain. Targets **Linux, macOS, Windows, Android, iOS** on **x86_64 & ARM**. Includes Option B (native‑only) notes for future research.

> Owner: `github.com/divine/secure-app-framework` (rename later if needed).  
> License: **Apache‑2.0 or MIT** (pick one; examples assume Apache‑2.0).  
> Status: **Test app** + **framework skeleton** suitable for production hardening.

---

## 0) Executive Summary

This repository demonstrates a **new delivery paradigm**: ship application logic as a **WebAssembly (WASM) component** compiled from **Rust**, and run it through a **small, signed native broker** per platform that:
- Provides **capability‑based filesystem access** (user‑granted folders only) and **network access** via **WASI Preview 2** host shims.
- Enforces **least privilege** + **policy** (allowlists/quotas) and produces a **tamper‑evident audit log**.
- Hosts a **native‑feeling UI** (Tauri v2 or Slint) for an ultra‑modern cross‑platform experience.

The framework comes with a **reproducible build toolchain**, **artifact signing (Sigstore cosign)**, **SLSA provenance**, **SBOM generation (CycloneDX / Syft)**, and **CI matrices** to cross‑compile and package installers for all target OSes. Security checks include **CodeQL** for Rust (public preview), **cargo‑audit** (RustSec), and **cargo‑deny** (licenses & bans).

The included **Test App** exercises the end‑to‑end flow: user picks a workspace folder, the broker **pre‑opens** it for the WASM core, the core **lists/reads/writes** files inside that folder, fetches remote JSON (network allowed domain), and renders data with a polished UI. Every FS/network operation is **logged & hash‑chained**.

> **Why Option A?** One auditable core across all targets, strict sandboxing by default, and a consistent security model.  
> **Option B** (native‑only) is documented below for scenarios needing deep OS integration or kernel‑level performance.

---

## 1) Goals & Non‑Goals

### Goals
- **Portability:** Ship one core logic component across all OS/CPU targets.
- **Sandboxing:** No ambient FS or network; **capabilities are explicitly granted**.
- **Reproducibility:** Deterministic builds + verifiable provenance.
- **Auditability:** SBOMs, signed artifacts, SLSA provenance, hash‑chained audit log.
- **Excellent UX:** First‑run permission prompts, polished UI, native‑feeling look.
- **Developer Experience:** Clear tasks, scripts, CI, and templates.

### Non‑Goals
- Full containerisation (not required for end‑users).
- Kernel‑mode drivers or hypervisor isolation (out of scope; can be added later).
- App Store publication guides (summarised but store‑submission specifics are separate).

---

## 2) Architecture Overview (Option A: WASM + Native Broker)

```
┌─────────────────────────────────────────────────────────────────┐
│                      Native Broker (Rust)                       │
│  UI (Tauri v2 or Slint)   |   WASI Host (Wasmtime)              │
│  • OS pickers (xdg-portal /  |  • Pre-open FS (granted dirs)    │
│    macOS bookmarks / Win FAL)|  • sockets (WASI p2)             │
│  • Policy engine (JSON)    |   • host bindings for WIT          │
│  • Hash-chained audit log  |   • audit hooks                    │
│  • OS sandbox (Landlock / AppContainer / App Sandbox)           │
└──────────────┬───────────────────────────────────────────┬──────┘
               │                                           │
         WIT bindings                                 User Grants
               │                                           │
        ┌──────▼───────────────────────────────────────────▼─────┐
        │                    WASM Core (Rust)                     │
        │   • Pure app logic (deterministic, capability-based)   │
        │   • Filesystem & network via WIT interfaces             │
        └─────────────────────────────────────────────────────────┘
```

### Key Concepts
- **WIT/Component model:** Define a `world` for `fs`, `net`, `log`, `ui-bridge` (minimal), implemented by the host. The core never performs raw syscalls.
- **Preopens & pickers:** The broker uses OS pickers to acquire user‑granted directories, then **pre‑opens** only those into the WASI FS.
- **Network:** The broker exposes **WASI Preview 2 sockets** (via Wasmtime) gated by policy (domain/IP allowlist, TLS by default).
- **Audit:** Every host call (FS/Net) is logged with a rolling hash (H2 = H(H1 || event)), persisted within the app data dir.
- **Sandbox:** The broker itself reduces ambient rights (Linux **Landlock**; Windows **AppContainer** via MSIX; macOS **App Sandbox**).

---

## 3) Components & Crate Layout

Monorepo workspace:

```
/secure-app-framework
├─ /crates
│  ├─ broker/            # Native host/broker (Rust, Wasmtime, UI shell)
│  ├─ core/              # Rust → WASM component (business logic)
│  ├─ wit/               # .wit files, shared interfaces & versions
│  ├─ ui/                # Tauri v2 (default) or Slint UI layer
│  ├─ policy/            # Policy JSON schemas + parser
│  └─ audit/             # Hash‑chained logging utilities
├─ /dist                 # Packaged artifacts (CI)
├─ /scripts              # Build, sign, verify, reproduce
├─ /ci                   # GitHub Actions workflows
├─ /docs                 # Additional docs (Option B notes, threat model)
└─ README.md
```

### Broker (Rust)
- Embeds **Wasmtime** with **WASI Preview 1 & 2** support.
- Implements WIT host traits for `fs`, `net`, `log`, `time`, `rand` (deterministic stub for tests).
- Calls OS pickers; persist grants:
  - **Linux:** `xdg-desktop-portal` FileChooser + **Documents portal** for durable access.
  - **Windows:** **FutureAccessList** (FAL) for persistent file/folder tokens.
  - **macOS/iOS:** **security‑scoped bookmarks** (`startAccessingSecurityScopedResource`).
  - **Android:** **Storage Access Framework (SAF)** (`DocumentFile`, tree URIs).
- Enforces policy: allowlists for domains, max payload size, path quotas; policy file in app config dir.
- Starts UI shell (Tauri v2 default; Slint as alternative) and bridges minimal events to the core.

### Core (Rust → WASM component)
- Pure logic compiled with `cargo component` using WIT for host calls.
- Implements app features (see Test App below). No platform code.

### UI (Tauri v2 default)
- UI written with web tech (HTML/CSS/TS) but packaged as a native app; or **Slint** for fully native widgets.
- Communicates with broker via command API; broker calls core and returns results.
- Themes: light/dark; native system font stack; high‑DPI assets; keyboard shortcuts.

---

## 4) Test App Functionality (MVP)

1) **Workspace Selection**: first run shows a platform picker; user chooses a folder.  
   - Broker persists the grant (FAL/bookmark/SAF) and **pre‑opens** it for the core under `/workspace`.
2) **Filesystem Operations** (via core):
   - List directory, open/edit/save text/JSON files **within** `/workspace` only.
   - Attempting to escape `/workspace` is blocked & logged.
3) **Network Fetch** (via core):
   - GET `https://example.org/data.json` (configurable allowlist). Display data in UI.
4) **Audit Panel**:
   - Real‑time, append‑only view of FS/Net events with rolling hash; export audit log.
5) **Policy & Permissions**:
   - UI page shows active grants and policy; allow user to revoke domain or folder grants.
6) **Telemetry**: off by default; opt‑in only; logs never leave device unless explicitly exported.

---

## 5) Security Model

- **Capability‑based runtime** (no ambient FS/Net): host grants **only what’s asked & approved**.
- **Defense‑in‑depth**: sandbox the broker itself (Landlock/AppContainer/App Sandbox). No JIT if policy forbids; prefer AOT compilation of WASM module where available.
- **Cryptography**: BLAKE3 for audit chaining; SHA‑256 for artifact checksums; TLS (native platform) for network. All crypto dependencies pinned.
- **Secrets**: OS keychain/keystore if needed. No secrets in source/CI logs.
- **Threats considered**: supply‑chain tampering, malicious dependency, path traversal, TOCTOU FS, exfiltration via network, downgrade of policy, binary substitution.
- **Out‑of‑scope (for MVP)**: kernel exploits, malicious UI themes, side channels, hardware attacks.

---

## 6) Build, Cross‑Compilation & Packaging

### Toolchain & Targets
- **Rust stable** + `cargo component` (WIT bindings) for the core.
- **Wasmtime** in broker; `wasmtime-wasi` p1/p2 host support.
- Targets (examples):  
  - macOS: `aarch64-apple-darwin`, `x86_64-apple-darwin`  
  - Windows: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`  
  - Linux: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`  
  - Android: via Tauri mobile or cargo‑mobile; AAB/APK (arm64-v8a)  
  - iOS: via Tauri mobile or cargo‑mobile; `aarch64-apple-ios`

> **Note:** On iOS you must build on macOS; on Windows ARM you’ll need the MSVC toolset; on Linux ARM cross use `cross`/`zig` or native runners.

### CI (GitHub Actions) Matrix (excerpt)
- Jobs: `build_core_wasm`, `build_broker_{windows,macos,linux}`, `package_desktop`, `sign_{win,mac,linux}`, `mobile_{android,ios}`, `sbom`, `provenance`, `release`.
- Cache: cargo, npm (for Tauri UI), wasmtime artifacts.
- Reproducible flags (see §7); release artifacts smoke‑tested on VMs.

### Desktop Packaging
- **Windows**: MSIX (preferred; AppContainer), or signed exe/msi.
- **macOS**: `.app` + `.dmg`, hardened runtime + notarisation.
- **Linux**: AppImage (signed), plus optional `.deb`/`.rpm` if desired.

### Mobile Packaging
- **Android**: AAB for Play, signed with `apksigner` (v2/v3/v4 as per minSdk).  
- **iOS**: Xcode project via Tauri mobile/cargo‑mobile; provisioning profiles; App Sandbox entitlements.

---

## 7) Reproducible Builds & Provenance

### Determinism
- Set `SOURCE_DATE_EPOCH` during builds.
- Disable timestamps & embed paths: use `RUSTFLAGS="--remap-path-prefix=$(pwd)=/source"`; prefer Cargo `trim-paths` if available.
- Pin toolchains via `rust-toolchain.toml`; lock dependencies (`Cargo.lock`) and npm `package-lock.json`/`pnpm-lock.yaml`.
- Avoid non‑determinism (time, randomness) in core logic; gate randomness behind deterministic PRNG for tests.

### SBOM & Vulnerability Scanning
- **CycloneDX for Cargo** (`cyclonedx-rust-cargo`) + **Syft** for binary SBOMs.  
- Continuous scanning with **Grype** (optional) and **cargo‑audit**.

### SLSA & Signing
- Generate **in‑toto/SLSA provenance** for every artifact.
- **Cosign**: sign release zips, installers, and SBOMs. Prefer **keyless** with GitHub OIDC.
- Publish checksums, SBOMs, signatures, and provenance in the GitHub Release.

---

## 8) Code Signing (per‑platform)

- **Windows**: Sign with **SignTool** (`/fd SHA256 /tr <timestamp> /td SHA256`). Prefer MSIX packaging for AppContainer benefits.
- **macOS**: `codesign` with **Hardened Runtime**, then **notarytool** submit; **staple** tickets; include entitlements.
- **Linux**: Sign AppImages with **gpg** (AppImage `--sign`); publish public key; provide verification instructions.
- **Android**: Sign with **apksigner**; configure **Play App Signing** for store deployment.
- **iOS**: Apple certificates & provisioning; entitlements aligned with App Sandbox.

Scripts in `/scripts` provide `sign-win.ps1`, `sign-mac.sh`, `sign-linux.sh`, `sign-android.sh`, `sign-ios.sh` examples.

---

## 9) UI & UX Guidelines

- **Look & feel**: native system font stack; adaptive spacing; dark/light theming; prefers‑color‑scheme on desktop; haptic/gesture support on mobile.
- **First‑run**: gentle primer + picker dialog; show what access is granted and why. Persist choices; easily revoke.
- **Status surface**: in‑app sheet shows: selected workspace path, domains allowed, and live audit events.
- **Accessibility**: keyboard navigation, high contrast, screen‑reader labels, scalable text.
- **Performance**: lazy load large folder listings; debounce IO; async tasks with progress HUDs.

---

## 10) Option B (Native‑Only) – Future Track

If deep OS integration or kernel‑level file performance is required, compile Rust **natively** per platform and apply OS sandboxes directly:
- **macOS**: App Sandbox entitlements + security‑scoped bookmarks; Hardened Runtime + notarisation.
- **Windows**: MSIX AppContainer; FutureAccessList persistence; WinUI UI.
- **Linux**: Landlock to drop ambient FS; XDG portals for user‑granted files.

You still keep §7–§8 supply‑chain guarantees (reproducible builds, SBOM, SLSA, signatures). The test app can be recompiled to native and reuse the same UI (Tauri/Slint).

---

## 11) Tasks (Detailed, end‑to‑end)

### Milestone 0: Repo & Scaffolding
- [ ] Create repo `secure-app-framework` with `LICENSE`, `CODE_OF_CONDUCT.md`, `SECURITY.md`.
- [ ] Add Rust workspace with crates: `broker`, `core`, `wit`, `ui`, `policy`, `audit`.
- [ ] Add rust‑toolchain, `.editorconfig`, `.gitattributes` (normalize line endings), `.pre-commit-config.yaml` (fmt, clippy).

### Milestone 1: WIT & Core
- [ ] Define `wit/world.wit` with interfaces: `fs`, `net`, `log`, `time`, `rand`.
- [ ] Add `core` crate using `cargo component`; implement functions:
      `list_dir(path)`, `read_text(path)`, `write_text(path, content)`, `fetch_json(url)`.
- [ ] Unit tests (deterministic PRNG, fixture FS via in‑memory adapter).

### Milestone 2: Broker Host & Policy
- [ ] Integrate **Wasmtime**; implement host bindings for the WIT world.
- [ ] Implement **preopen** logic; map `/workspace` to granted dir(s).
- [ ] OS pickers:
    - Linux: call **xdg‑desktop‑portal** FileChooser; persist via Documents portal.
    - Windows: FolderPicker + **FutureAccessList**.
    - macOS/iOS: NSOpenPanel / UIDocumentPicker + **security‑scoped bookmarks**.
    - Android: **SAF** (ACTION_OPEN_DOCUMENT_TREE) → persistable URI permissions.
- [ ] Policy engine: JSON (allowlisted domains, max bytes, path quotas, timeouts).
- [ ] Audit log crate: append‑only file with rolling hash (BLAKE3), rotation, export.

### Milestone 3: UI Shell
- [ ] Tauri v2 UI (default): setup routes/panels (Workspace, Files, Network, Audit, Policy).
- [ ] Native‑feel theming; keyboard shortcuts; file editor component.
- [ ] Connect UI → broker commands; broker → core; render results & errors.

### Milestone 4: Cross‑Build & Packaging
- [ ] GitHub Actions matrix (win/macos/linux; x86_64/arm64) builds broker+core, packages installers.
- [ ] Android (AAB/APK) & iOS builds using Tauri mobile or cargo‑mobile; device smoke tests.
- [ ] Upload artifacts; generate checksums.

### Milestone 5: Reproducibility & Security
- [ ] Make builds deterministic: `SOURCE_DATE_EPOCH`, `--remap-path-prefix`, pinned toolchains.
- [ ] SBOMs: CycloneDX for Cargo + Syft for produced binaries.
- [ ] Vulnerability scanning: **cargo‑audit** (RustSec), **cargo‑deny** (licenses/bans).
- [ ] **CodeQL** for Rust (public preview) in CI; schedule weekly deep scans.
- [ ] SLSA provenance (in‑toto attestation) for all artifacts.
- [ ] Sign artifacts with **cosign** (keyless via GitHub OIDC).

### Milestone 6: Code Signing & Notarisation
- [ ] Windows: Sign installers (and binaries) with **SignTool**; MSIX optional.
- [ ] macOS: Hardened Runtime, entitlements, `notarytool` submit & **staple**.
- [ ] Linux: AppImage **gpg** signature; publish public key and verification instructions.
- [ ] Android: sign with **apksigner**; enable Play App Signing.
- [ ] iOS: configure certificates & provisioning; entitlements aligned with sandbox.

### Milestone 7: QA & Release
- [ ] End‑to‑end tests: pick workspace → read/write → network fetch → audit export.
- [ ] Fuzz core APIs with `cargo‑fuzz`; property tests with `proptest`.
- [ ] Manual UX review on each platform (HIG checks).
- [ ] Tag `v0.1.0-alpha` release with SBOMs, signatures, provenance, installers.

---

## 12) Scripts & Snippets (Examples)

### Deterministic build environment
```bash
# scripts/env.sh
export SOURCE_DATE_EPOCH="$(git log -1 --pretty=%ct)"
export RUSTFLAGS="--remap-path-prefix=$(pwd)=/source"
export CARGO_TERM_COLOR=never
```

### Cosign (keyless) sign/verify
```bash
# Sign
cosign sign-blob --yes --identity-token "$ACTIONS_ID_TOKEN"   --output-signature dist/app.zip.sig dist/app.zip

# Verify (public transparency log)
cosign verify-blob --certificate-oidc-issuer https://token.actions.githubusercontent.com   --signature dist/app.zip.sig dist/app.zip
```

### Windows SignTool
```powershell
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 `
  /a ".\dist\AppInstaller.msi"
```

### macOS notarisation
```bash
xcrun codesign --force --options runtime --entitlements entitlements.plist   --sign "Developer ID Application: YOUR ORG" MyApp.app
xcrun notarytool submit MyApp.dmg --apple-id you@example.com --team-id ABCDE12345 --wait
xcrun stapler staple MyApp.app
```

### SBOM (CycloneDX + Syft)
```bash
cargo install cyclonedx-bom
cyclonedx-bom -o sbom-cyclonedx.json

syft dir:. -o cyclonedx-json > binary-sbom.json
```

---

## 13) References & Further Reading

- **WASI/Wasmtime & Component Model**
  - Wasmtime WASI p2 host: https://docs.wasmtime.dev/api/wasmtime_wasi/p2/
  - Component model & WIT (Rust): https://docs.wasmtime.dev/api/wasmtime/component/
  - Component model tutorial: https://component-model.bytecodealliance.org/tutorial.html
- **Turning WASM into native executables**
  - Wasmer “WASM as universal binary” & `create-exe`: https://wasmer.io/posts/wasm-as-universal-binary-format-part-1-native-executables
  - Wasmer create‑exe docs: https://wasmerio.github.io/wasmer/crates/doc/wasmer_cli/commands/create_exe/
- **OS‑level pickers & persistent access**
  - Windows **FutureAccessList**: https://learn.microsoft.com/en-us/uwp/api/windows.storage.accesscache.storageitemaccesslist
  - macOS **security‑scoped bookmarks**: https://developer.apple.com/documentation/foundation/url/startaccessingsecurityscopedresource
  - Android **SAF**: https://developer.android.com/guide/topics/providers/document-provider
  - XDG **FileChooser** & Documents portal: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html
- **OS sandboxes**
  - Linux **Landlock**: https://docs.kernel.org/userspace-api/landlock.html
  - Windows **AppContainer**: https://learn.microsoft.com/en-us/windows/msix/msix-container
  - macOS **App Sandbox**: https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.app-sandbox
- **UI frameworks**
  - **Tauri v2** (desktop+mobile): https://v2.tauri.app/
  - **Slint**: https://slint.dev/
- **Reproducible builds & provenance**
  - Reproducible builds: https://reproducible-builds.org/
  - SOURCE_DATE_EPOCH: https://reproducible-builds.org/docs/source-date-epoch/
  - Cargo path sanitisation (trim‑paths/remap): https://rust-lang.github.io/rfcs/3127-trim-paths.html
  - SLSA provenance: https://slsa.dev/spec/v0.1/provenance
  - Sigstore Cosign quickstart: https://docs.sigstore.dev/quickstart/quickstart-cosign/
  - Syft (SBOM): https://github.com/anchore/syft
  - CycloneDX Cargo: https://github.com/CycloneDX/cyclonedx-rust-cargo
- **Security scanners**
  - cargo‑audit (RustSec): https://crates.io/crates/cargo-audit
  - cargo‑deny: https://github.com/EmbarkStudios/cargo-deny
  - GitHub CodeQL for Rust: https://github.blog/changelog/2025-06-30-codeql-support-for-rust-now-in-public-preview/
- **Windows & macOS signing**
  - SignTool: https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool
  - macOS Notarization: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
  - AppImage signing: https://docs.appimage.org/packaging-guide/optional/signatures.html
  - Android signing: https://developer.android.com/studio/publish/app-signing

---

## 14) Contributing & Governance

- All changes via PR with CI green. Security‑relevant changes require two reviewers.
- Run `scripts/checks.sh` (fmt, clippy, deny, audit) before pushing.
- Security policy in `SECURITY.md` explains vulnerability reporting.

---

## 15) Roadmap (beyond MVP)

- Sandboxed plugin system (untrusted extensions in separate WASM modules).
- AOT‑compiled WASM for faster startup, WAMR/Cranelift exploration.
- Optional TEE‑based build attestations.
- Auto‑update channel secured with TUF.

---

## 16) Appendix: Threat Model (MVP extract)

**Assets:** user workspace contents; credentials/tokens; integrity of binaries; privacy.  
**Adversaries:** malicious dependencies; compromised CI; local malware; network MITM.  
**Mitigations:** deterministic builds, SBOMs, signatures, code scanning, strict capability grants, OS sandboxes, TLS‑only networking, human review of policy changes.

---

> **Kickoff:** create repo, copy this README, and open issues for each milestone. The CI and boilerplate templates will follow in the first PR.

