# nametag

`nametag` is a tiny Windows utility that shows the name of the currently logged-on user in a small banner pinned to the top of the screen.

The point of it is to provide an **on-screen visual indicator of who is logged on**, which is useful in schools and other managed deployments where machines are shared and it is helpful to see at a glance which account is in use (for example in screenshots, screen recordings, or when supporting a user remotely).

## Behaviour

- Shows the Windows username in a small, always-on-top banner at the top-centre of the primary screen.
- The banner does not appear in the taskbar or Alt-Tab.
- Drag it left/right with the mouse to move it out of the way.
- **Ctrl+Alt+click** on the banner to exit the program.

## Download

[Latest `nametag.exe` (built from `main`)](https://github.com/mrtckirby/nametag/raw/main/target/x86_64-pc-windows-gnu/release/nametag.exe)

## Building

The program targets Windows (`x86_64-pc-windows-gnu`) and uses the `windows-sys` crate.

### Windows (VS Code or VSCodium)

1. Install [Rust via rustup](https://rustup.rs/) and the GNU toolchain target:
   ```
   rustup target add x86_64-pc-windows-gnu
   ```
   You need a MinGW-w64 toolchain providing `x86_64-w64-mingw32-gcc` on your `PATH` (for example via [MSYS2](https://www.msys2.org/): `pacman -S mingw-w64-x86_64-gcc`).
2. Install [VS Code](https://code.visualstudio.com/) or [VSCodium](https://vscodium.com/) and the **rust-analyzer** extension.
3. Open the project folder (`File > Open Folder...`).
4. Open the integrated terminal (`Ctrl+\``) and run:
   ```
   cargo build --release --target x86_64-pc-windows-gnu
   ```
5. The executable is at `target\x86_64-pc-windows-gnu\release\nametag.exe`.

### Linux (VS Code or VSCodium, cross-compiling)

1. Install Rust with [rustup](https://rustup.rs/) and add the target:
   ```
   rustup target add x86_64-pc-windows-gnu
   ```
2. Install the MinGW-w64 cross compiler, e.g. on Debian/Ubuntu:
   ```
   sudo apt install mingw-w64
   ```
3. Install VS Code or VSCodium and the **rust-analyzer** extension (on VSCodium, install it from Open VSX).
4. Open the project folder (`File > Open Folder...`).
5. Open the integrated terminal (`Ctrl+\``) and run:
   ```
   cargo build --release --target x86_64-pc-windows-gnu
   ```
6. The executable is at `target/x86_64-pc-windows-gnu/release/nametag.exe`. The linker is configured in `.cargo/config.toml`.

Because the code is Windows-only, rust-analyzer on Linux may report errors for the Win32 APIs; set `"rust-analyzer.cargo.target": "x86_64-pc-windows-gnu"` in your workspace settings to fix this.
