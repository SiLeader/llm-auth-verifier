# llm-auth-verifier

[English](README.md) | [日本語](README_ja.md)

`llm-auth-verifier` は、OpenAI API および Anthropic Messages API 互換エンドポイントを提供するローカル LLM サーバー（vLLM、Ollama、llama.cpp、LocalAI など）を保護するための、軽量かつ高速なアクセストークン認証サービスです。

[Caddy ウェブサーバー](https://caddyserver.com/) の [`forward_auth`](https://caddyserver.com/docs/caddyfile/directives/forward_auth) ディレクティブと組み合わせて使用することを前提として設計されています。Caddy がリバースプロキシとしてアップストリームのローカル LLM にリクエストを転送する前に、クライアント認証の検証を `llm-auth-verifier` に委譲します。

---

## 目次

- [特徴](#特徴)
- [動作の仕組み](#動作の仕組み)
- [対応エンドポイントと認証ヘッダー](#対応エンドポイントと認証ヘッダー)
- [インストール](#インストール)
- [使い方](#使い方)
  - [コマンドラインオプション](#コマンドラインオプション)
  - [Caddy 設定の生成](#caddy-設定の生成)
- [設定](#設定)
  - [静的トークン設定](#静的トークン設定)
  - [OpenID Connect (OIDC) / JWT 設定](#openid-connect-oidc--jwt-設定)
  - [`tokens.toml` の完全な設定例](#tokenstoml-の完全な設定例)
- [Caddy との連携](#caddy-との連携)
  - [`Caddyfile` の設定例](#caddyfile-の設定例)
- [テストとリクエスト例](#テストとリクエスト例)
- [ライセンス](#ライセンス)

---

## 特徴

- **OpenAI & Anthropic API 互換**: OpenAI 互換 API および Anthropic Messages API 互換エンドポイントの両方の認証リクエストを判定可能。
- **複数のトークン形式**: 平文（`raw`）、SHA-256（`sha256`）、SHA-512（`sha512`）のハッシュ形式に対応。
- **タイミング攻撃対策**: 一定時間比較（`constant_time_eq`）によるセキュアな文字列検証。
- **トークンの有効期限**: トークンごとに有効期限（`expire_at`）を設定可能。
- **API スコープ制限**: トークンごとに `openai` または `anthropic` のエンドポイントに限定、あるいは両方で利用可能に設定可能。
- **OpenID Connect (OIDC) / JWT 対応**: OIDC Discovery（`/.well-known/openid-configuration`）および JWKS によるキー自動取得・キャッシュ・ローテーション対応の JWT 検証。
- **Caddy Forward Auth 設定の自動出力**: Caddyfile 用の `forward_auth` 設定スニペットを出力する CLI コマンド（`--caddy`）を搭載。

---

## 動作の仕組み

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
               200 OK (認可成功)       |    401 Unauthorized (認可失敗)
          +---------------------------+---------------------------+
          |                                                       |
          v                                                       v
+-------------------+                                   +-------------------+
| アップストリームへ転送  |                                   |  リクエストを拒否    |
| (例: :8000のLLM)  |                                   |   (HTTP 401)      |
+-------------------+                                   +-------------------+
```

1. クライアントが LLM エンドポイント（`/v1/chat/completions` や `/v1/messages` など）に向けて Caddy リバースプロキシにリクエストを送信します。
2. Caddy は `forward_auth` により、リクエストヘッダーを保持したまま `llm-auth-verifier` の `/verify` エンドポイントへ認可確認を行います。
3. `llm-auth-verifier` は以下を検証します:
   - `X-Forwarded-Uri` ヘッダーから元のリクエスト URI を判定し、アクセス先が OpenAI API か Anthropic API かを特定。
   - `Authorization: Bearer <token>` または `x-api-key: <token>` からトークンを抽出。
4. トークンが有効であり、有効期限内で対象 API と一致する場合（または JWT/OIDC 検証をパスした場合）、HTTP `200 OK` を返します。
5. Caddy は認証成功を受けてリクエストをローカル LLM にプロキシ転送します。認証に失敗した場合は HTTP `401 Unauthorized` を返してアクセスを遮断します。

---

## 対応エンドポイントと認証ヘッダー

### エンドポイント

| 対象 API | 認識されるリクエスト URI |
|---|---|
| **Anthropic Messages API** | `/v1/messages` |
| **OpenAI API** | `/v1/models`, `/v1/responses`, `/v1/chat/completions`, `/v1/embeddings`, `/v1/completions` |

### 認証ヘッダー

- **OpenAI API**:
  - `Authorization: Bearer <token>`（大文字・小文字不問）
- **Anthropic Messages API**:
  - `Authorization: Bearer <token>`
  - `x-api-key: <token>`

---

## インストール

### 前提条件

- [Rust](https://www.rust-lang.org/) (2024 edition 以降)
- `cargo`

### ソースコードからのビルド

```bash
git clone https://github.com/SiLeader/llm-auth-verifier.git
cd llm-auth-verifier
cargo build --release
```

ビルドが完了すると、バイナリは `target/release/llm-auth-verifier` に生成されます。

---

## 使い方

### コマンドラインオプション

```bash
llm-auth-verifier [OPTIONS]
```

| オプション | デフォルト値 | 説明 |
|---|---|---|
| `--listen <LISTEN>` | `127.0.0.1:9731` | バインドするホストとポート |
| `--tokens <TOKENS>` | `/etc/llm-auth-verifier/tokens.toml` | トークン設定ファイル（TOML）のパス |
| `--caddy` | - | Caddy の `forward_auth` 設定ブロックを出力して終了 |
| `-h, --help` | - | ヘルプメッセージを表示 |

### サーバーの起動

```bash
# 設定ファイルのパスとリッスンアドレスを指定して起動
llm-auth-verifier --listen 127.0.0.1:9731 --tokens /path/to/tokens.toml
```

### Caddy 設定の出力

Caddyfile に記載すべきディレクティブをコマンドから出力できます:

```bash
llm-auth-verifier --listen 127.0.0.1:9731 --caddy
```

出力例:
```caddy
forward_auth 127.0.0.1:9731 {
    uri /verify
}
```

---

## 設定

設定ファイルは TOML 形式で記述します。**静的トークン（Static Tokens）** と **OpenID Connect (OIDC)** の 2 種類の設定に対応しています。

### 静的トークン設定

各 `[[tokens]]` エントリでトークンルールを定義します:

- `raw` *(文字列, 省略可)*: トークンの平文字列。
- `sha256` *(文字列, 省略可)*: トークンの小文字 64 文字 16 進数 SHA-256 ハッシュ。
- `sha512` *(文字列, 省略可)*: トークンの小文字 128 文字 16 進数 SHA-512 ハッシュ。
- `api` *(文字列, 省略可)*: トークンの対象 API を `"openai"` または `"anthropic"` に限定。省略時は両方で有効。
- `expire_at` *(文字列, 省略可)*: RFC 3339 形式の有効期限日時（例: `2026-12-31T23:59:59Z`）。

> **注意**: 各トークンエントリには `raw`、`sha256`、`sha512` のうち少なくとも 1 つの指定が必須です。平文シークレットの保存を避けるため、ハッシュ化トークン（`sha256` または `sha512`）の使用を推奨します。

#### トークンハッシュの生成方法

```bash
# SHA-256 ハッシュ:
echo -n "my-secret-token" | sha256sum | cut -d' ' -f1

# SHA-512 ハッシュ:
echo -n "my-secret-token" | sha512sum | cut -d' ' -f1
```

### OpenID Connect (OIDC) / JWT 設定

各 `[[oidc]]` エントリで OIDC プロバイダーに対する JWT 検証を設定します:

- `issuer` *(文字列, 必須)*: OIDC Issuer URL（`/.well-known/openid-configuration` を提供している必要があります）。
- `audiences` *(文字列配列, 必須)*: 許容する `aud` クレーム一覧。
- `subjects` *(文字列配列, 必須)*: 許容する `sub` クレーム一覧。
- `cache_ttl` *(文字列, 省略可)*: JWKS キャッシュの有効期間（例: `"12h"`, `"1h"`, `"30m"`）。デフォルトは `12h`。

### `tokens.toml` の完全な設定例

```toml
# OpenAI / Anthropic の両方で使用できる平文トークン
[[tokens]]
raw = "sk-plain-text-token-12345"

# 有効期限付きの SHA-256 ハッシュトークン
[[tokens]]
sha256 = "4c5dc9b7708905f77f5e5d16316b5dfb425e68cb326dcd55a860e90a7707031e"
expire_at = "2026-12-31T23:59:59Z"

# OpenAI API エンドポイントのみに限定した SHA-512 ハッシュトークン
[[tokens]]
sha512 = "1fb3d3b3ed263ff715b48dfad17cc9e69697ccc59ba7c57922c7bc5e5312494542b788e22ce84463678e266e71ce0c401c9bdef9587b7c2a9d7dca4b38a031e8"
api = "openai"

# Anthropic API エンドポイントのみに限定したトークン
[[tokens]]
raw = "sk-ant-restricted-token"
api = "anthropic"

# OpenID Connect (JWT) 検証
[[oidc]]
issuer = "https://auth.example.com"
audiences = ["llm-service"]
subjects = ["user-123", "service-account-abc"]
cache_ttl = "6h"
```

---

## Caddy との連携

### `Caddyfile` の設定例

ポート `8000` で稼働しているローカル LLM サーバーを保護する Caddyfile の例です:

```caddy
llm.example.com {
    # llm-auth-verifier に認証を委譲
    forward_auth 127.0.0.1:9731 {
        uri /verify
    }

    # ローカル LLM サーバー（vLLM, Ollama, llama.cpp 等）へリバースプロキシ
    reverse_proxy 127.0.0.1:8000
}
```

---

## テストとリクエスト例

### 1. OpenAI Chat Completions リクエスト

```bash
curl -X POST https://llm.example.com/v1/chat/completions \
  -H "Authorization: Bearer sk-plain-text-token-12345" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-ai/DeepSeek-R1-Distill-Qwen-32B",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### 2. Anthropic Messages リクエスト

`x-api-key` ヘッダーを使用する場合:
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

`Authorization: Bearer` ヘッダーを使用する場合:
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

### 3. 認証失敗時

不正なトークン、期限切れトークン、または未対応のパスに対するリクエストは拒否されます:
```
HTTP/1.1 401 Unauthorized
```

---

## ライセンス

本プロジェクトは [Apache License 2.0](LICENSE) の下で公開されています。
