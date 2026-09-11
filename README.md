# llm-auth-verifier

[English](README.md) | [日本語](README_ja.md)

`llm-auth-verifier` is a lightweight authentication service for local LLM servers that expose OpenAI- or Anthropic-compatible APIs, including llama.cpp, Ollama, LM Studio, and vLLM.

It runs behind a reverse proxy. The proxy sends each request to `/verify`, and the verifier checks the original HTTP method and path together with a static access token or an OpenID Connect (OIDC) JWT. Only recognized LLM API endpoints are authorized.

## Features

- OpenAI- and Anthropic-compatible endpoint detection by HTTP method and path
- Built-in API presets for llama.cpp, Ollama, LM Studio, and vLLM
- Exact-path and regular-expression definitions for custom APIs
- Plaintext, SHA-256, and SHA-512 static token representations
- Constant-time token comparison, optional expiration, and per-API scoping
- OIDC discovery and JWT verification with JWKS caching and key rotation
- Configuration generators for Caddy, Traefik, and nginx
- JSON logs with authentication audit fields
- A minimal, non-root container image

## How it works

```text
Client
  |
  | LLM API request
  v
Reverse proxy ---- auth request ----> llm-auth-verifier /verify
  |                                      |
  | 2xx: authorized                      | checks X-Forwarded-Method,
  |                                      | X-Forwarded-Uri, and the token
  v                                      |
Local LLM server                    401: rejected
```

The reverse proxy must pass the original request in these headers:

- `X-Forwarded-Method`: the original HTTP method
- `X-Forwarded-Uri`: the original request URI; query parameters are ignored during API matching
- `Authorization: Bearer <token>`: accepted for every configured API
- `x-api-key: <token>`: also accepted for predefined Anthropic APIs

The verifier returns `200 OK` when both the endpoint and credential are allowed. Invalid credentials, expired credentials, missing forwarding headers, and unmatched methods or paths return `401 Unauthorized`.

## Installation

### Build from source

Install a Rust toolchain with Rust 2024 edition support, then run:

```bash
git clone https://github.com/SiLeader/llm-auth-verifier.git
cd llm-auth-verifier
cargo build --release
```

The binary is written to `target/release/llm-auth-verifier`.

### Container image

The project publishes `ghcr.io/sileader/llm-auth-verifier`. Mount the configuration read-only at the default path:

```bash
docker run --rm \
  -p 9731:9731 \
  -v "$PWD/config.toml:/etc/llm-auth-verifier/config.toml:ro" \
  ghcr.io/sileader/llm-auth-verifier:latest
```

The image listens on `0.0.0.0:9731` and runs as UID/GID `1000:1000`, so the mounted file must be readable by that user.

## Usage

```text
Usage: llm-auth-verifier [OPTIONS] [COMMAND]

Commands:
  caddy-config
  traefik-config
  nginx-config
  predefined-apis

Options:
      --listen <LISTEN>  Listen host and port [default: 127.0.0.1:9731]
      --config <CONFIG>  Path to configuration file [default: /etc/llm-auth-verifier/config.toml]
  -h, --help             Print help
```

Start the server with a custom configuration:

```bash
llm-auth-verifier --listen 127.0.0.1:9731 --config /path/to/config.toml
```

Set `RUST_LOG` to adjust log filtering. For example:

```bash
RUST_LOG=llm_auth_verifier=debug llm-auth-verifier --config ./config.toml
```

The process handles Ctrl+C and, on Unix, `SIGTERM` for graceful shutdown.

## Configuration

The TOML configuration has three top-level areas: `[api]`, `[[tokens]]`, and `[[oidc]]`.

### API selection

Select one built-in provider preset:

```toml
[api]
provider = "v-llm" # llama-cpp, ollama, lm-studio, or v-llm
```

If the entire `[api]` section is omitted, the `llama-cpp` preset is used. Each preset enables only the methods and paths implemented by that provider. See [predefined-apis.md](predefined-apis.md), or inspect them from the installed binary:

```bash
llm-auth-verifier predefined-apis
llm-auth-verifier predefined-apis --provider v-llm
```

Use `named_apis` to add selected predefined APIs. An `[api]` section without `provider` can therefore act as an allowlist:

```toml
[api]
named_apis = [
  "openai/chat-completions",
  "anthropic/messages",
]
```

Custom endpoints support exact or regex path matching:

```toml
[api]
provider = "ollama"

[[api.custom_apis]]
name = "custom/rerank"
method = "POST"
path = "/v1/rerank"
path_type = "exact"

[[api.custom_apis]]
name = "custom/model-files"
method = "GET"
path = "^/models/[0-9]+/files$"
path_type = "regex"
```

`name` is optional for a custom API and defaults to `<METHOD>:<PATH>`. Custom APIs accept Bearer credentials, but are not assigned to the OpenAI or Anthropic API family; use `allowed_apis` rather than the legacy `api` field to scope tokens to them.

### Static tokens

Each `[[tokens]]` entry accepts these fields. At least one of `raw`, `sha256`, or `sha512` is required.

| Field | Required | Description |
|---|---:|---|
| `name` | No | Name included in audit logs; defaults to `index:<n>`. |
| `raw` | Conditional | Plaintext token. |
| `sha256` | Conditional | Hex-encoded SHA-256 digest. |
| `sha512` | Conditional | Hex-encoded SHA-512 digest. If multiple token values are present, this takes precedence, followed by `sha256`, then `raw`. |
| `allowed_apis` | No | API names this token may access. Omit to allow every configured API. |
| `api` | No | Legacy family scope: `openai` or `anthropic`. |
| `expire_at` | No | RFC 3339 expiration timestamp. |

Hashed token storage is recommended:

```bash
printf %s "my-secret-token" | sha256sum | cut -d' ' -f1
printf %s "my-secret-token" | sha512sum | cut -d' ' -f1
```

Example:

```toml
[[tokens]]
name = "chat-client"
sha256 = "ea5add57437cbf20af59034d7ed17968dcc56767b41965fcc5b376d45db8b4a3"
allowed_apis = ["openai/chat-completions", "openai/models"]
expire_at = "2026-12-31T23:59:59Z"

[[tokens]]
name = "anthropic-client"
raw = "sk-ant-restricted-token"
api = "anthropic"
```

### OpenID Connect / JWT

Each `[[oidc]]` entry configures one issuer:

| Field | Required | Description |
|---|---:|---|
| `issuer` | Yes | Issuer URL. Its discovery document must be available at `/.well-known/openid-configuration`. |
| `audiences` | Yes | Accepted `aud` claim values. |
| `algorithms` | Yes | Accepted JWT signature algorithms, such as `RS256`, `ES256`, or `HS256`. |
| `access_rules` | Yes | Claim-based API access rules. At least one rule is required. |
| `cache_ttl` | No | JWKS cache lifetime such as `30m`, `1h`, or `12h`; defaults to `12h`. |

```toml
[[oidc]]
issuer = "https://auth.example.com"
audiences = ["llm-service"]
algorithms = ["RS256", "ES256"]
cache_ttl = "6h"

[[oidc.access_rules]]
claims = { sub = "user-123" }
allowed_apis = ["openai/chat-completions"]

[[oidc.access_rules]]
claims = { department = "research", groups = "llm-admins" }
allowed_apis = ["openai/chat-completions", "openai/models"]
```

Every claim in an access rule must match, and any matching rule can grant access to the requested API. A scalar expected value also matches an element in an array claim, so `groups = "llm-admins"` matches `"groups": ["users", "llm-admins"]`. Use dot-separated selectors such as `realm.role`, or a JSON Pointer such as `/https:~1~1example.com~1roles/0`, for nested claims. String, number, boolean, array, and object values are supported.

JWTs must include `kid`, `aud`, `iss`, and `exp`. A rule can require `sub`, but tokens matched through other claims do not need it. The verifier loads RSA, EC, HMAC (`oct`), and EdDSA (`OKP`) keys from JWKS during startup, refreshes stale keys, and attempts a rate-limited refresh when it encounters an unknown `kid`.

### Complete example

```toml
[api]
provider = "ollama"
named_apis = ["openai/audio-transcriptions"]

[[api.custom_apis]]
name = "custom/rerank"
method = "POST"
path = "/v1/rerank"
path_type = "exact"

[[tokens]]
name = "application"
raw = "replace-with-a-secret"
allowed_apis = ["openai/chat-completions", "custom/rerank"]

[[oidc]]
issuer = "https://auth.example.com"
audiences = ["llm-service"]
algorithms = ["RS256"]
cache_ttl = "12h"

[[oidc.access_rules]]
claims = { sub = "user-123" }
allowed_apis = ["openai/chat-completions", "custom/rerank"]
```

## Reverse proxy integration

`--listen` controls both the server bind address and the address printed by configuration-generator commands. Place this global option before the subcommand.

### Caddy

```bash
llm-auth-verifier --listen 127.0.0.1:9731 caddy-config
```

```caddy
llm.example.com {
    forward_auth 127.0.0.1:9731 {
        uri /verify
    }

    reverse_proxy 127.0.0.1:8000
}
```

### nginx

Generate the internal authentication location:

```bash
llm-auth-verifier --listen 127.0.0.1:9731 nginx-config
```

Then reference it with `auth_request` in the protected location:

```nginx
location = /__/llm-auth-verifier/verify {
    internal;
    proxy_pass http://127.0.0.1:9731/verify;
    proxy_set_header X-Forwarded-Uri $request_uri;
    proxy_set_header X-Forwarded-Method $request_method;
}

location / {
    auth_request /__/llm-auth-verifier/verify;
    proxy_pass http://127.0.0.1:8000;
}
```

### Traefik

Generate dynamic middleware configuration in YAML, TOML, Docker labels, Consul Catalog tags, or Kubernetes CRD format:

```bash
llm-auth-verifier --listen llm-auth-verifier:9731 traefik-config
llm-auth-verifier --listen llm-auth-verifier:9731 traefik-config --format kubernetes --name llm-auth
```

Run `llm-auth-verifier traefik-config --help` for all formats and options, then attach the generated middleware to the router that fronts the LLM server.

## Request examples

OpenAI-compatible request:

```bash
curl https://llm.example.com/v1/chat/completions \
  -H "Authorization: Bearer replace-with-a-secret" \
  -H "Content-Type: application/json" \
  -d '{"model":"example-model","messages":[{"role":"user","content":"Hello"}]}'
```

Anthropic-compatible request:

```bash
curl https://llm.example.com/v1/messages \
  -H "x-api-key: sk-ant-restricted-token" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{"model":"example-model","max_tokens":128,"messages":[{"role":"user","content":"Hello"}]}'
```

## License

Licensed under the [Apache License 2.0](LICENSE).
