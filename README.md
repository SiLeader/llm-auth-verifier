# llm-auth-verifier

[English](README.md) | [日本語](README_ja.md)

`llm-auth-verifier` is a lightweight, high-performance authentication service designed to secure local LLM servers (such as vLLM, Ollama, llama.cpp, LocalAI, etc.) exposing OpenAI API and Anthropic Messages API compatible endpoints.

It is designed to be used in conjunction with the [Caddy web server](https://caddyserver.com/)'s [`forward_auth`](https://caddyserver.com/docs/caddyfile/directives/forward_auth) directive. Caddy delegates client authorization checks to `llm-auth-verifier` before proxying requests to the upstream local LLM engine.

---

## Table of Contents

- [Features](#features)
- [How It Works](#how-it-works)
- [Supported Endpoints & Authentication Headers](#supported-endpoints--authentication-headers)
- [Installation](#installation)
- [Usage](#usage)
  - [Command-line Options](#command-line-options)
  - [Generating Caddy Configuration](#generating-caddy-configuration)
- [Configuration](#configuration)
  - [Static Token Configuration](#static-token-configuration)
  - [OpenID Connect (OIDC) / JWT Configuration](#openid-connect-oidc--jwt-configuration)
  - [Complete `tokens.toml` Example](#complete-tokenstoml-example)
- [Caddy Integration](#caddy-integration)
  - [Example `Caddyfile`](#example-caddyfile)
- [Testing & Examples](#testing--examples)
- [License](#license)

---

## Features

- **OpenAI & Anthropic API Compatibility**: Validates requests for both OpenAI-compatible and Anthropic Messages API compatible endpoints.
- **Multiple Token Hash Algorithms**: Supports plain-text (`raw`), SHA-256 (`sha256`), and SHA-512 (`sha512`) token representations.
- **Constant-Time Comparison**: Protects against timing attacks using constant-time string comparison (`constant_time_eq`).
- **Token Expiration**: Set an optional expiration timestamp (`expire_at`) per token.
- **API-Level Scoping**: Restrict tokens to either `openai` or `anthropic` endpoints, or allow both.
- **OpenID Connect (OIDC) / JWT Support**: Verify JWTs dynamically using OIDC discovery (`/.well-known/openid-configuration`) and JWKS with automatic key caching and rotation.
- **Native Caddy Forward Auth Integration**: Built-in CLI command (`--caddy`) to output the exact `forward_auth` block for your `Caddyfile`.

---

## How It Works

```
                        +---------------------------+
                        |  Client (SDK, WebUI, CLI) |
                        +-------------+-------------+
                                      |
                                      | HTTP Request (OpenAI / Anthropic API)
                                      v
                        +---------------------------+
                        |     Caddy Reverse Proxy   |
                        +-------------+-------------+
                                      |
                         forward_auth | GET /verify
                                      v
                        +---------------------------+
                        |     llm-auth-verifier     |
                        +-------------+-------------+
                                      |
               200 OK (Authorized)    |    401 Unauthorized
          +---------------------------+---------------------------+
          |                                                       |
          v                                                       v
+-------------------+                                   +-------------------+
| Proxy to Upstream |                                   |  Reject Request   |
|     Local LLM     |                                   |   (HTTP 401)      |
|  (e.g., :8000)    |                                   +-------------------+
+-------------------+
```

1. The client sends a request to the Caddy reverse proxy targeting an LLM endpoint (e.g., `/v1/chat/completions` or `/v1/messages`).
2. Caddy uses `forward_auth` to forward verification information to `llm-auth-verifier` at `/verify`.
3. `llm-auth-verifier` inspects:
   - The original request URI via `X-Forwarded-Uri` to identify whether the target API is OpenAI or Anthropic.
   - The token from `Authorization: Bearer <token>` or `x-api-key: <token>`.
4. If the token is valid, unexpired, and matches the target API (or passes JWT/OIDC validation), `llm-auth-verifier` responds with HTTP `200 OK`.
5. Caddy then proxies the request to the upstream local LLM server. Otherwise, it returns HTTP `401 Unauthorized`.

---

## Supported Endpoints & Authentication Headers

### Endpoints

| Target API | Recognized Request URIs |
|---|---|
| **Anthropic Messages API** | `/v1/messages` |
| **OpenAI API** | `/v1/models`, `/v1/responses`, `/v1/chat/completions`, `/v1/embeddings`, `/v1/completions` |

### Authentication Headers

- **OpenAI API**:
  - `Authorization: Bearer <token>` (case-insensitive scheme)
- **Anthropic Messages API**:
  - `Authorization: Bearer <token>`
  - `x-api-key: <token>`

---

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/) (2024 edition or newer)
- `cargo`

### Build from Source

```bash
git clone https://github.com/SiLeader/llm-auth-verifier.git
cd llm-auth-verifier
cargo build --release
```

The compiled binary will be located at `target/release/llm-auth-verifier`.

---

## Usage

### Command-line Options

```bash
llm-auth-verifier [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--listen <LISTEN>` | `127.0.0.1:9731` | Host and port to listen on. |
| `--tokens <TOKENS>` | `/etc/llm-auth-verifier/tokens.toml` | Path to the configuration TOML file. |
| `--caddy` | - | Print the Caddy `forward_auth` configuration block and exit. |
| `-h, --help` | - | Display help information. |

### Running the Server

```bash
# Using a custom tokens configuration path and listen port
llm-auth-verifier --listen 127.0.0.1:9731 --tokens /path/to/tokens.toml
```

### Generating Caddy Configuration

You can print the required Caddy snippet directly:

```bash
llm-auth-verifier --listen 127.0.0.1:9731 --caddy
```

Output:
```caddy
forward_auth 127.0.0.1:9731 {
    uri /verify
}
```

---

## Configuration

The configuration file is written in TOML format and supports two authentication mechanisms: **Static Tokens** and **OpenID Connect (OIDC)**.

### Static Token Configuration

Each `[[tokens]]` entry defines a token rule:

- `raw` *(string, optional)*: Plaintext token.
- `sha256` *(string, optional)*: 64-character lowercase hexadecimal SHA-256 hash of the token.
- `sha512` *(string, optional)*: 128-character lowercase hexadecimal SHA-512 hash of the token.
- `api` *(string, optional)*: Restricts the token to `"openai"` or `"anthropic"`. If omitted, the token is valid for both.
- `expire_at` *(string, optional)*: RFC 3339 formatted expiration timestamp (e.g. `2026-12-31T23:59:59Z`).

> **Note**: At least one of `raw`, `sha256`, or `sha512` is required per token entry. Storing hashed tokens (`sha256` or `sha512`) is recommended to avoid saving plaintext secrets in the config file.

#### Generating Token Hashes

```bash
# SHA-256 hash:
echo -n "my-secret-token" | sha256sum | cut -d' ' -f1

# SHA-512 hash:
echo -n "my-secret-token" | sha512sum | cut -d' ' -f1
```

### OpenID Connect (OIDC) / JWT Configuration

Each `[[oidc]]` entry configures JWT authentication against an OpenID Connect provider:

- `issuer` *(string, required)*: OIDC issuer URL (must provide `/.well-known/openid-configuration`).
- `audiences` *(array of strings, required)*: Valid `aud` claims.
- `subjects` *(array of strings, required)*: Allowed `sub` claims.
- `cache_ttl` *(string, optional)*: Time-to-live for cached JWKS keys (e.g., `"12h"`, `"1h"`, `"30m"`). Defaults to `12h`.

### Complete `tokens.toml` Example

```toml
# Plain-text token valid for both OpenAI and Anthropic
[[tokens]]
raw = "sk-plain-text-token-12345"

# SHA-256 hashed token with an expiration date
[[tokens]]
sha256 = "4c5dc9b7708905f77f5e5d16316b5dfb425e68cb326dcd55a860e90a7707031e"
expire_at = "2026-12-31T23:59:59Z"

# SHA-512 hashed token restricted to OpenAI API endpoints only
[[tokens]]
sha512 = "1fb3d3b3ed263ff715b48dfad17cc9e69697ccc59ba7c57922c7bc5e5312494542b788e22ce84463678e266e71ce0c401c9bdef9587b7c2a9d7dca4b38a031e8"
api = "openai"

# Token restricted to Anthropic API endpoints only
[[tokens]]
raw = "sk-ant-restricted-token"
api = "anthropic"

# OpenID Connect (JWT) validation
[[oidc]]
issuer = "https://auth.example.com"
audiences = ["llm-service"]
subjects = ["user-123", "service-account-abc"]
cache_ttl = "6h"
```

---

## Caddy Integration

### Example `Caddyfile`

Here is an example Caddy configuration that secures a local LLM server running on port `8000`:

```caddy
llm.example.com {
    # Forward auth verification to llm-auth-verifier
    forward_auth 127.0.0.1:9731 {
        uri /verify
    }

    # Reverse proxy to the local LLM server (vLLM, Ollama, llama.cpp, etc.)
    reverse_proxy 127.0.0.1:8000
}
```

---

## Testing & Examples

### 1. OpenAI Chat Completions Request

```bash
curl -X POST https://llm.example.com/v1/chat/completions \
  -H "Authorization: Bearer sk-plain-text-token-12345" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-ai/DeepSeek-R1-Distill-Qwen-32B",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### 2. Anthropic Messages Request

Using `x-api-key`:
```bash
curl -X POST https://llm.example.com/v1/messages \
  -H "x-api-key: sk-ant-restricted-token" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet",
    "max_tokens": 1024,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

Or using `Authorization: Bearer`:
```bash
curl -X POST https://llm.example.com/v1/messages \
  -H "Authorization: Bearer sk-plain-text-token-12345" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet",
    "max_tokens": 1024,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### 3. Unauthorized Requests

Any request with an invalid token, expired token, or unsupported path will receive:
```
HTTP/1.1 401 Unauthorized
```

---

## License

This project is licensed under the [Apache License 2.0](LICENSE).
