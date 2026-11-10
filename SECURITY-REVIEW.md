# Security and stability review

**Reviewed:** 2026-11-10
**Repository revision:** `fccd9d1236782d9f376021f44918b5f81a803f67`
**Scope:** Source, Cargo manifests and lockfile, build configuration, and checked-in build output.

## Summary

No confirmed exploitable security vulnerability was identified in the reviewed source. The program appears to be a small, user-level Windows overlay: it displays the current Windows account name and reads the current user's Windows theme setting. The reviewed code contains no network communication, credential handling, privilege elevation, automatic startup configuration, or registry writes.

This is a visual label, not a persistent or tamper-resistant control. A logged-in user can close it or terminate the process, and the repository does not configure it to launch or restart. The committed executable also has no demonstrated link to a reviewed source build. These make the project unsuitable as-is for enforcing identity display on school computers.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | Medium — operational/requirements risk, not a vulnerability | The banner can be closed and does not restart itself. It is an ordinary topmost popup; closing it destroys the window and ends the message loop. There is no installer or startup/relaunch configuration in the repository. | [`src/main.rs`](src/main.rs#L78-L91), [`src/main.rs`](src/main.rs#L224-L229) |
| 2 | Medium — artifact trust risk, not evidence of malware | A Windows executable is checked into `target/`, but the repository has no published release or build workflow, and the artifact has no demonstrated signature or build-provenance record. The reviewed source therefore cannot be assumed to match that executable. | `target/x86_64-pc-windows-gnu/release/nametag.exe`; no release or workflow configuration found |

## Stability and compatibility

- Several Win32 operations are used without checking their return values, including class registration, GDI device-context/font creation, text measurement, window creation, and applying the window region (`src/main.rs`, approximately lines 43–91). If an API call fails, the program may proceed with invalid or zero geometry, show a malformed banner, or fail without a useful diagnostic. In particular, the code does not check whether `CreateWindowExW` succeeded before calling `SetWindowRgn`.
- The window state is transferred to a raw pointer before window creation and normally reclaimed in `WM_NCDESTROY` (`src/main.rs`, lines 69–76 and 224–227). A window-creation failure path is not explicitly handled, so resource cleanup and user-visible error reporting are not robustly defined for that case.
- The banner uses primary-display dimensions for initial placement and drag limits (`src/main.rs`, lines 66–67 and 203–215). Multi-monitor layouts, display changes, and scaling should be tested on the intended machines; the program does not account for the full virtual desktop.
- The username API failure path displays the generic string `User` (`src/main.rs`, lines 249–257). That is a safe fallback, but could silently show an incorrect label.
- There are no tests or CI workflows in the repository, so automated verification and Windows runtime behavior are not demonstrated.

## Security and privacy notes

- The displayed account name is visible to anyone looking at the screen (`src/main.rs`, lines 23–24 and 182–188). If school usernames encode student IDs or other identifying information, treat that as an intentional disclosure and confirm it is acceptable.
- The overlay is not a security boundary. It does not authenticate the user, prevent switching applications, or stop someone from closing the window or ending the process.
- The theme lookup reads `AppsUseLightTheme` under `HKEY_CURRENT_USER` (`src/main.rs`, lines 233–247). The reviewed code does not write registry settings.
- The source depends on `windows-sys` 0.59 and `Cargo.lock` pins package versions and checksums, which supports repeatable dependency resolution. It does not establish the provenance of the checked-in executable. The build configuration selects the `x86_64-pc-windows-gnu` target and requires the MinGW linker (`.cargo/config.toml`).

## Recommendation

Use only as a supervised visual aid after a school-controlled build from a reviewed source revision. Verify the resulting executable, sign and track its hash, and pilot it with the school's account-name conventions, display layouts, and scaling settings. If it must appear after login, deploy it through school-managed policy; if users must not be able to remove it, use an appropriate managed kiosk or shell policy rather than relying on this application.
