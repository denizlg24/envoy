# Envoy

**Envoy** is a secure, Git-like CLI for managing encrypted environment files across machines and teams.

It lets you **encrypt once**, **sync safely**, and **restore automatically** — without ever committing secrets to Git.

---

## Why Envoy?

Managing `.env` files across devices is painful:

- You can’t commit them
- Copying them manually is error-prone
- Sharing them securely is hard
- CI environments need controlled access

Envoy solves this by treating secrets like **versioned artifacts**, not source code.

---

## Core Concepts

Envoy is built around a few simple ideas:

- **Secrets are encrypted locally** (never plaintext on the server)
- **Encrypted blobs are content-addressed** (SHA-256)
- **Git tracks intent, not data**
- **Cache is disposable**
- **Remotes behave like Git remotes**

If you understand Git, Envoy will feel familiar.

---

## Installation

### Quick Install

**macOS/Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/denizlg24/envoy/master/install.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/denizlg24/envoy/master/install.ps1 | iex
```

### From Source

**Requirements**
- Rust (stable)
- Cargo

```bash
cargo install envoy-cli
```

Or build locally:

```bash
cargo build --release
```

---

## Authentication

Envoy uses GitHub OAuth (device flow).

```bash
envy login
```

This stores an API token in:

```
$HOME/.envoy/config.toml
```

Logout at any time:

```bash
envy logout
```

---

## Getting Started

### 1. Initialize a project

```bash
envy init
```

This creates the `.envoy/` directory and sets up the default remote (`origin`).

### 2. Choose files to encrypt

Add files (default `.env`) using your workflow.  
Secrets are tracked internally and never committed to Git.

```bash
envy encrypt
envy encrypt --input .env.testing
```

### 3. Push encrypted secrets

```bash
envy push
```

- Encrypts files locally
- Uploads encrypted blobs
- Updates the manifest

### 4. Pull and restore secrets

```bash
envy pull
```

- Downloads encrypted blobs
- Decrypts them locally
- Restores files to their original paths

---

## Status

Check whether your local cache is in sync:

```bash
envy status
```

This command is:
- offline-safe
- read-only
- fast

---

## Configuration

### Project config (tracked)

`.envoy/config.toml`

```toml
version = 1
project_id = "..."
default_remote = "origin"

[remotes]
origin = "https://envoy-cli.vercel.app/api"
```

---

## Security Model

- Encryption happens **client-side**
- Keys are derived using **Argon2id**
- Data is encrypted using **XChaCha20-Poly1305**
- Blobs are verified via **SHA-256**
- Server never sees plaintext
- Cache can be deleted at any time

Envoy is designed so the server is **untrusted by default**.

---

## License

MIT
