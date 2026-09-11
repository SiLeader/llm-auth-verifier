# llm-auth-verifier

[English](README.md) | [日本語](README_ja.md)

`llm-auth-verifier` は、llama.cpp、Ollama、LM Studio、vLLM など、OpenAI または Anthropic 互換 API を提供するローカル LLM サーバー向けの軽量な認証サービスです。

リバースプロキシの背後で動作し、プロキシから `/verify` に送られたリクエストについて、元の HTTP メソッドとパス、および静的アクセストークンまたは OpenID Connect（OIDC）JWT を検証します。定義済みの LLM API エンドポイントだけを認可します。

## 特徴

- HTTP メソッドとパスによる OpenAI／Anthropic 互換エンドポイントの判定
- llama.cpp、Ollama、LM Studio、vLLM の API プリセット
- 完全一致または正規表現によるカスタム API 定義
- 平文、SHA-256、SHA-512 形式の静的トークン
- 定数時間比較、有効期限、API ごとのアクセス制限
- OIDC Discovery と、JWKS のキャッシュ・キーローテーションに対応した JWT 検証
- Caddy、Traefik、nginx 向け設定の生成コマンド
- 認証監査フィールドを含む JSON ログ
- 非 root ユーザーで動作する最小構成のコンテナイメージ

## 動作の仕組み

```text
クライアント
  |
  | LLM API リクエスト
  v
リバースプロキシ ---- 認証リクエスト ----> llm-auth-verifier /verify
  |                                        |
  | 2xx: 認可成功                          | X-Forwarded-Method、
  |                                        | X-Forwarded-Uri、トークンを検証
  v                                        |
ローカル LLM サーバー                 401: 拒否
```

リバースプロキシは、元のリクエスト情報を次のヘッダーで渡す必要があります。

- `X-Forwarded-Method`: 元の HTTP メソッド
- `X-Forwarded-Uri`: 元のリクエスト URI（API 判定時にクエリパラメーターは無視されます）
- `Authorization: Bearer <token>`: 設定したすべての API で利用可能
- `x-api-key: <token>`: 定義済み Anthropic API でも利用可能

エンドポイントと認証情報の両方が許可されている場合は `200 OK` を返します。不正または期限切れの認証情報、転送ヘッダーの不足、未定義のメソッドやパスは `401 Unauthorized` になります。

## インストール

### ソースコードからビルド

Rust 2024 edition に対応した Rust ツールチェーンをインストールし、次を実行します。

```bash
git clone https://github.com/SiLeader/llm-auth-verifier.git
cd llm-auth-verifier
cargo build --release
```

バイナリは `target/release/llm-auth-verifier` に生成されます。

### コンテナイメージ

`ghcr.io/sileader/llm-auth-verifier` が公開されています。設定ファイルをデフォルトパスに読み取り専用でマウントします。

```bash
docker run --rm \
  -p 9731:9731 \
  -v "$PWD/config.toml:/etc/llm-auth-verifier/config.toml:ro" \
  ghcr.io/sileader/llm-auth-verifier:latest
```

イメージは `0.0.0.0:9731` で待ち受け、UID/GID `1000:1000` で動作します。マウントするファイルには、このユーザーからの読み取り権限が必要です。

## 使い方

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

任意の設定ファイルを指定してサーバーを起動します。

```bash
llm-auth-verifier --listen 127.0.0.1:9731 --config /path/to/config.toml
```

ログのフィルターは `RUST_LOG` で変更できます。例:

```bash
RUST_LOG=llm_auth_verifier=debug llm-auth-verifier --config ./config.toml
```

Ctrl+C、および Unix では `SIGTERM` を受け取るとグレースフルシャットダウンします。

## 設定

TOML 設定ファイルは、トップレベルの `[api]`、`[[tokens]]`、`[[oidc]]` で構成されます。

### API の選択

組み込みのプロバイダープリセットを 1 つ選択します。

```toml
[api]
provider = "v-llm" # llama-cpp、ollama、lm-studio、v-llm のいずれか
```

`[api]` セクション全体を省略した場合は `llama-cpp` プリセットが使われます。各プリセットでは、そのプロバイダーが実装するメソッドとパスだけが有効になります。詳細は [predefined-apis.md](predefined-apis.md) または次のコマンドで確認できます。

```bash
llm-auth-verifier predefined-apis
llm-auth-verifier predefined-apis --provider v-llm
```

`named_apis` を使うと、定義済み API を個別に追加できます。`provider` のない `[api]` セクションは許可リストとして利用できます。

```toml
[api]
named_apis = [
  "openai/chat-completions",
  "anthropic/messages",
]
```

カスタムエンドポイントでは、完全一致と正規表現によるパスマッチを利用できます。

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

カスタム API の `name` は省略でき、その場合は `<METHOD>:<PATH>` になります。カスタム API は Bearer 認証を受け付けますが、OpenAI／Anthropic の API ファミリーには分類されません。トークンのアクセス範囲をカスタム API に限定する場合は、従来の `api` ではなく `allowed_apis` を使用してください。

### 静的トークン

各 `[[tokens]]` エントリで次のフィールドを指定できます。`raw`、`sha256`、`sha512` のうち、少なくとも 1 つが必要です。

| フィールド | 必須 | 説明 |
|---|---:|---|
| `name` | いいえ | 監査ログに記録する名前。省略時は `index:<n>`。 |
| `raw` | 条件付き | 平文トークン。 |
| `sha256` | 条件付き | 16 進数で表した SHA-256 ダイジェスト。 |
| `sha512` | 条件付き | 16 進数で表した SHA-512 ダイジェスト。複数のトークン値を指定した場合は `sha512`、`sha256`、`raw` の順に優先。 |
| `allowed_apis` | いいえ | このトークンでアクセスできる API 名。省略時は設定済みの全 API を許可。 |
| `api` | いいえ | 従来の API ファミリー制限。`openai` または `anthropic`。 |
| `expire_at` | いいえ | RFC 3339 形式の有効期限。 |

ハッシュ化したトークンの保存を推奨します。

```bash
printf %s "my-secret-token" | sha256sum | cut -d' ' -f1
printf %s "my-secret-token" | sha512sum | cut -d' ' -f1
```

設定例:

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

各 `[[oidc]]` エントリで 1 つの Issuer を設定します。

| フィールド | 必須 | 説明 |
|---|---:|---|
| `issuer` | はい | Issuer URL。`/.well-known/openid-configuration` で Discovery Document を提供する必要があります。 |
| `audiences` | はい | 許可する `aud` クレーム。 |
| `subjects` | いいえ | 許可する `sub` クレーム。省略時は任意の Subject を許可。 |
| `cache_ttl` | いいえ | JWKS キャッシュ期間。`30m`、`1h`、`12h` など。デフォルトは `12h`。 |

```toml
[[oidc]]
issuer = "https://auth.example.com"
audiences = ["llm-service"]
subjects = ["user-123", "service-account-abc"]
cache_ttl = "6h"
```

JWT には `kid`、`sub`、`aud`、`iss`、`exp` が必要です。起動時にキーを取得し、キャッシュが古くなった場合に更新します。未知の `kid` を検出した場合も、レート制限付きで更新を試みます。

### 完全な設定例

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
cache_ttl = "12h"
```

## リバースプロキシとの連携

`--listen` は、サーバーの待ち受けアドレスと、設定生成コマンドが出力する接続先の両方に使われます。このグローバルオプションはサブコマンドより前に指定してください。

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

内部認証用の location を生成します。

```bash
llm-auth-verifier --listen 127.0.0.1:9731 nginx-config
```

保護する location から `auth_request` で参照します。

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

YAML、TOML、Docker labels、Consul Catalog tags、Kubernetes CRD 形式の動的ミドルウェア設定を生成できます。

```bash
llm-auth-verifier --listen llm-auth-verifier:9731 traefik-config
llm-auth-verifier --listen llm-auth-verifier:9731 traefik-config --format kubernetes --name llm-auth
```

全形式とオプションは `llm-auth-verifier traefik-config --help` で確認できます。生成したミドルウェアを LLM サーバーの前段にある Router に設定してください。

## リクエスト例

OpenAI 互換リクエスト:

```bash
curl https://llm.example.com/v1/chat/completions \
  -H "Authorization: Bearer replace-with-a-secret" \
  -H "Content-Type: application/json" \
  -d '{"model":"example-model","messages":[{"role":"user","content":"Hello"}]}'
```

Anthropic 互換リクエスト:

```bash
curl https://llm.example.com/v1/messages \
  -H "x-api-key: sk-ant-restricted-token" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" \
  -d '{"model":"example-model","max_tokens":128,"messages":[{"role":"user","content":"Hello"}]}'
```

## ライセンス

[Apache License 2.0](LICENSE) の下で公開されています。
