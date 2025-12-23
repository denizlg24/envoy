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

## Architecture Overview

```
.envoy/
├─ config.toml        # project metadata (tracked)
├─ latest             # manifest pointer (tracked)
└─ cache/             # encrypted blobs (ignored)
```

- Only `config.toml` and `latest` are committed
- Encrypted data lives in object storage
- The server never sees plaintext

---

## Installation

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

Add files (e.g. `.env`, `.env.local`) using your workflow.  
Secrets are tracked internally and never committed to Git.

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

## Remotes

Envoy supports Git-style remotes.

```bash
envy remote add origin http://localhost:3000
envy remote add prod https://api.envoy.dev
```

Use a specific remote:

```bash
envy push prod
envy pull prod
```

If omitted, Envoy uses the default remote.

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
origin = "http://localhost:3000"
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

## Commands

```bash
envy login
envy logout
envy init
envy push [remote]
envy pull [remote]
envy status
envy remote add <name> <url>
```

---

## Current Status

Envoy is currently **v0.1**.

- Core workflow complete
- CLI stable
- APIs subject to change
- No team sharing yet

---

## Roadmap

- `envy whoami`
- `envy remote list`
- Garbage collection
- CI-friendly auth
- Team projects
- Conflict detection

---

## License

MIT
