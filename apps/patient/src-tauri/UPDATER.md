# Auto-Updater Setup for Monobase Patient

This document describes how to set up the auto-updater for Monobase Patient desktop builds.

## Overview

The auto-updater uses:
- **minisign** for cryptographic signature verification
- **Tauri Plugin Updater** for update checks and installation
- **CDN** for hosting update artifacts

## 1. Generate Signing Keys

Install minisign:
```bash
# macOS
brew install minisign

# Linux
sudo apt install minisign

# Windows (via scoop)
scoop install minisign
```

Generate a keypair:
```bash
# Generate keys (keep tauri.key SECRET, share tauri.pub)
minisign -G -p tauri.pub -s tauri.key

# You will be prompted for a password to protect the secret key
```

The command creates:
- `tauri.pub` - Public key (add to tauri.conf.json)
- `tauri.key` - Private key (use in CI/CD, KEEP SECRET)

## 2. Configure tauri.conf.json

Add the public key to `tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "active": true,
      "dialog": false,
      "endpoints": [
        "https://downloads.monobase.com/patient/{{target}}_{{arch}}/latest.json"
      ],
      "pubkey": "YOUR_PUBLIC_KEY_HERE"
    }
  }
}
```

## 3. CDN Structure

Set up the following structure on your CDN:

```
https://downloads.monobase.com/patient/
├── darwin_aarch64/
│   ├── latest.json
│   └── Monobase_Patient_x.x.x_aarch64.app.tar.gz
│       └── .sig (signature file)
├── darwin_x86_64/
│   ├── latest.json
│   └── Monobase_Patient_x.x.x_x64.app.tar.gz
│       └── .sig
├── windows_x86_64/
│   ├── latest.json
│   └── Monobase_Patient_x.x.x_x64_en-US.msi
│       └── .sig
└── linux_x86_64/
    ├── latest.json
    └── Monobase_Patient_x.x.x_amd64.AppImage
        └── .sig
```

### latest.json Format

```json
{
  "version": "1.0.0",
  "notes": "Release notes here...",
  "pub_date": "2025-01-15T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "BASE64_SIGNATURE_HERE",
      "url": "https://downloads.monobase.com/patient/darwin_aarch64/Monobase_Patient_1.0.0_aarch64.app.tar.gz"
    }
  }
}
```

## 4. CI/CD Integration

### Sign Artifacts

In your CI/CD pipeline, sign the built artifacts:

```bash
# Set the secret key password in environment
export MINISIGN_PASSWORD="your_secret_key_password"

# Sign the artifact (creates .sig file)
minisign -S -s tauri.key -m Monobase_Patient_1.0.0_aarch64.app.tar.gz
```

### GitHub Actions Example

```yaml
name: Build and Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - platform: macos-latest
            target: aarch64-apple-darwin
          - platform: macos-latest
            target: x86_64-apple-darwin
          - platform: windows-latest
            target: x86_64-pc-windows-msvc
          - platform: ubuntu-latest
            target: x86_64-unknown-linux-gnu

    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4

      - name: Build Tauri App
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: v__VERSION__
          releaseName: 'Monobase Patient v__VERSION__'
          releaseBody: 'See the assets to download this version and install.'
          releaseDraft: true
          prerelease: false
```

### Environment Variables

Set these secrets in your CI/CD:
- `TAURI_SIGNING_PRIVATE_KEY`: Contents of `tauri.key` file
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: Password used when generating the key

## 5. Manual Update Check

The app checks for updates:
1. On startup (silent, automatic)
2. Via system tray menu → "Check for Updates" (manual, shows dialogs)

See `src/lib.rs::check_for_updates()` for implementation.

## 6. Testing Updates

For local testing:

1. Build version 1.0.0 and install
2. Increment version to 1.0.1 in package.json
3. Build and sign the new version
4. Host latest.json locally (e.g., with `npx serve`)
5. Temporarily modify endpoints in tauri.conf.json to point to localhost
6. Run the installed app and trigger update check

## Security Notes

- **NEVER** commit `tauri.key` to version control
- Store the private key and password in secure secret management
- Rotate keys periodically
- Verify signatures are checked before installation (handled by Tauri)
