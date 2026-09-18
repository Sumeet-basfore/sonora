# Sonora Installation Guide

This guide explains how to install and run **Sonora** on Linux, Windows, and macOS without building from source code.

> Download official release packages from: **[github.com/Sumeet-basfore/sonora/releases/latest](https://github.com/Sumeet-basfore/sonora/releases/latest)**

---

## 1. Desktop GUI

### Linux

#### Option A: AppImage (Universal — Recommended)
Works on all major desktop distributions (Ubuntu, Debian, Fedora, Arch, openSUSE, Linux Mint).

1. Download `Sonora-vX.Y.Z-linux-x86_64.AppImage`.
2. Make it executable:
   ```sh
   chmod +x Sonora-v*-linux-x86_64.AppImage
   ```
3. Run Sonora:
   ```sh
   ./Sonora-v*-linux-x86_64.AppImage
   ```
   *(Or double-click the AppImage file in your file manager).*

#### Option B: Debian / Ubuntu Package (`.deb`)
Native package for Ubuntu, Debian, Linux Mint, and Pop!_OS.

1. Download `Sonora-vX.Y.Z-linux-x86_64.deb`.
2. Install via terminal:
   ```sh
   sudo dpkg -i Sonora-v*-linux-x86_64.deb
   # If any system dependencies are missing, resolve them with:
   sudo apt-get install -f
   ```
3. Launch **Sonora** from your application menu or run `sonora-desktop` in terminal.

---

### Windows

1. Download `Sonora-vX.Y.Z-windows-x86_64.msi` (or the `.exe` installer).
2. Double-click the installer and follow the setup wizard.
3. Launch **Sonora** from the Start Menu or Desktop shortcut.

---

### macOS

1. Download `Sonora-vX.Y.Z-macos-aarch64.dmg` (Apple Silicon M1/M2/M3) or `Sonora-vX.Y.Z-macos-x86_64.dmg` (Intel).
2. Double-click the `.dmg` file.
3. Drag **Sonora.app** into your **Applications** folder.
4. Open Sonora from Spotlight or Launchpad.

> *Note on macOS:* If macOS displays a notice that the developer cannot be verified, right-click (or Control-click) `Sonora.app` in Finder and choose **Open**, then confirm.

---

## 2. Terminal Applications (CLI & TUI)

Sonora includes two standalone command-line tools in a single archive:
* `sonora`: Scriptable Command-Line Interface (CLI)
* `sonora-tui`: Full-screen interactive Terminal User Interface (TUI)

### Linux & macOS

1. Download the terminal archive for your system:
   * Linux: `Sonora-vX.Y.Z-linux-x86_64-terminal.tar.gz`
   * macOS (Apple Silicon): `Sonora-vX.Y.Z-macos-aarch64-terminal.tar.gz`
   * macOS (Intel): `Sonora-vX.Y.Z-macos-x86_64-terminal.tar.gz`
2. Extract the archive:
   ```sh
   tar -xzf Sonora-v*-terminal.tar.gz
   cd sonora-terminal
   ```
3. Run the tools directly:
   ```sh
   # Interactive terminal player:
   ./sonora-tui

   # CLI status and scan commands:
   ./sonora --help
   ./sonora status
   ./sonora scan /path/to/music
   ```
4. *(Optional)* Add to your system PATH:
   ```sh
   # User installation (no sudo):
   mkdir -p ~/.local/bin
   cp sonora sonora-tui ~/.local/bin/
   # Ensure ~/.local/bin is in your PATH in ~/.bashrc or ~/.zshrc

   # System-wide installation:
   sudo cp sonora sonora-tui /usr/local/bin/
   ```

---

### Windows

1. Download `Sonora-vX.Y.Z-windows-x86_64-terminal.zip`.
2. Right-click the `.zip` file and select **Extract All...**.
3. Open Command Prompt (`cmd.exe`) or PowerShell inside the extracted folder:
   ```powershell
   # Run interactive TUI:
   .\sonora-tui.exe

   # Run CLI commands:
   .\sonora.exe status
   .\sonora.exe scan "C:\Users\YourName\Music"
   ```
4. *(Optional)* Move `sonora.exe` and `sonora-tui.exe` to a permanent folder (such as `C:\Tools\Sonora`) and add it to your Windows User PATH in **Environment Variables**.

---

## 3. Verifying Downloads (SHA-256)

Every release includes a `checksums.txt` file on the release page.

### Linux / macOS:
```sh
sha256sum -c checksums.txt
# Or check a single file:
sha256sum Sonora-v0.1.0-linux-x86_64.AppImage
```

### Windows (PowerShell):
```powershell
Get-FileHash Sonora-v0.1.0-windows-x86_64.msi -Algorithm SHA256
```
Compare the output against the hash listed in `checksums.txt`.
