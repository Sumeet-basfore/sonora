# Sonora: Extension & Theme Marketplace Architecture

## 1. Marketplace Architecture Overview

Sonora adopts a decentralized, Git-backed community registry model inspired by Obsidian and Homebrew Cask. The architecture avoids centralized proprietary lock-in while providing fast, reliable discovery, cryptographically verified integrity, and automated security validation.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        MARKETPLACE ARCHITECTURE TOPOLOGY                    │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │               Community GitHub Registry Repository                  │   │
│   │         (https://github.com/sonora-audio/community-registry)        │   │
│   │   • plugins.json (Index of all verified plugin manifests)           │   │
│   │   • themes.json (Index of all verified theme manifests)             │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                                      ▼ (Automated CI / CDN Sync)             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    Global Fastly / Cloudflare CDN                   │   │
│   │  • https://registry.sonora.audio/v1/plugins.json                    │   │
│   │  • https://registry.sonora.audio/v1/themes.json                     │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                                      ▼ (HTTPS / Cache)                       │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    Sonora Client Marketplace View                   │   │
│   │   • Search, Category Filtering, Dynamic Popularity Sorting          │   │
│   │   • 1-Click Install, Auto-Update, and Rollback Manager              │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │ (Download Direct from Release Asset)  │
│                                      ▼                                       │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │           Developer GitHub / GitLab Release (Asset Tarball)         │   │
│   │   • plugin.wasm / index.js / manifest.json / signature.sig          │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Registry Data Format & Manifest Index

The central index files (`plugins.json`, `themes.json`) are lightweight JSON catalogs cached locally by the Sonora client:

```json
[
  {
    "id": "org.sonora.visualizer.projectm",
    "name": "Milkdrop 3 (projectM)",
    "description": "Hardware-accelerated classic Milkdrop visualizer presets for Sonora.",
    "author": { "name": "projectM Team", "url": "https://github.com/projectM-visualizer" },
    "repo": "https://github.com/sonora-community/sonora-projectm",
    "category": "visualizer",
    "latestVersion": "2.1.0",
    "minSonoraVersion": "1.0.0",
    "downloads": 48200,
    "stars": 1240,
    "releases": {
      "2.1.0": {
        "downloadUrl": "https://github.com/sonora-community/sonora-projectm/releases/download/v2.1.0/projectm.zip",
        "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "signature": "MC4CAQAwBgYDK2VwBQQEc3b0...Ed25519Sig..."
      }
    }
  }
]
```

---

## 3. Package Lifecycle: Install, Update & Rollback

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PACKAGE INSTALLATION WORKFLOW                       │
│                                                                             │
│   [ User Clicks Install ] ──► [ Download Release Tarball ]                  │
│                                           │                                 │
│                                           ▼                                 │
│   [ Reject & Notify ] ◄── [ Invalid ] ── [ Verify SHA-256 & Signature ]     │
│                                           │                                 │
│                                           ▼ (Valid)                         │
│   [ Reject & Cleanup ] ◄── [ Denied ] ── [ Prompt Capability Permissions ]  │
│                                           │                                 │
│                                           ▼ (User Approved)                 │
│   [ Unpack to $CONFIG/plugins/ ] ──► [ Instantiate Sandbox ] ──► [ Active ] │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Security & Verification Steps
1. **Download & Checksum**: Client downloads the tarball from the author's release URL and computes the SHA-256 hash.
2. **Signature Verification**: Validates the payload against the author's registered Ed25519 public key.
3. **Permission Review**: Displays explicit capability permissions (e.g. `Access to network domain https://api.spotify.com`) before unpacking.
4. **Non-Destructive Rollback**: When upgrading, the previous version is retained in a `.backup/` folder. If the new version crashes during initialization, Sonora automatically rolls back to the previous stable release.

---

## 4. Automated Submission & CI Validation Pipeline

```
Developer Submits PR to community-registry
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     AUTOMATED GITHUB ACTIONS CI RUNNER                      │
│                                                                             │
│  1. Manifest Schema Validation ($schema JSON Schema check)                  │
│  2. SemVer Version Hierarchy Verification                                   │
│  3. Asset Availability & HTTPS URL Check                                    │
│  4. SHA-256 Checksum Validation against Downloaded Release                  │
│  5. Static Code Analysis (Scans for eval(), obfuscated strings, etc.)       │
│  6. Headless Test Execution in Sandbox Environment                          │
└─────────────────────────────────────────────────────────────────────────────┘
                       │
                       ▼
            [ Automated / Review Merge ] ──► [ Re-generate Index CDN ]
```

---

## 5. Custom & Private Registry Support

Users and organizations can add custom repository URLs in settings:
- **Private Registries**: Point to internal GitLab / Gitea servers (`https://git.internal.corp/sonora-registry.json`).
- **Offline Mode**: Air-gapped workstations can install `.sonora-plugin` or `.sonora-theme` zip archives directly via drag-and-drop or command line (`sonora-cli plugin install ./my-plugin.zip`).

---

## 6. Reversibility & Marketplace Boundaries

> [!IMPORTANT]
> ### Reversible Architecture Decisions
> - **Registry Hosting**: The static JSON registry can be mirrored across GitHub Pages, Cloudflare R2, AWS S3, or IPFS without modifying client logic.
> - **Package Format**: Standard `.zip` / `.tar.gz` containers ensure extensions can be extracted and audited with standard system utilities.
>
> ### Invariable Boundaries
> - **Cryptographic Integrity**: All automated marketplace downloads must include a valid SHA-256 checksum in the index manifest.
