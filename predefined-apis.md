# 定義済み API リスト (Predefined APIs)

`llm-auth-verifier` にあらかじめ定義されている API 一覧と、各 LLM プロバイダーごとの対応状況および設定方法です。

---

## 目次

- [概要](#概要)
- [定義済み API 一覧](#定義済み-api-一覧)
  - [OpenAI 互換 API](#openai-互換-api)
  - [Anthropic 互換 API](#anthropic-互換-api)
- [プロバイダー別対応 API 一覧](#プロバイダー別対応-api-一覧)
  - [llama-cpp](#llama-cpp)
  - [ollama](#ollama)
  - [lm-studio](#lm-studio)
  - [v-llm](#v-llm)
- [設定方法](#設定方法)
  - [プロバイダーの一括指定](#プロバイダーの一括指定)
  - [個別 API の有効化 (named_apis)](#個別-api-の有効化-named_apis)
- [CLI コマンドでの確認](#cli-コマンドでの確認)

---

## 概要

`llm-auth-verifier` では、主要なローカル LLM プロバイダー（`llama-cpp`, `ollama`, `lm-studio`, `v-llm`）が提供するエンドポイントを事前定義しています。
設定ファイル（`config.toml`）で `provider` を指定することで対応する API 群が一括で有効化されるほか、`named_apis` を用いて特定の API のみを個別に追加・有効化することも可能です。

---

## 定義済み API 一覧

### OpenAI 互換 API

| API 名 (`name`) | メソッド | パス (`path`) | マッチ方式 | 対応プロバイダー |
|---|---|---|---|---|
| `openai/chat-completions` | `POST` | `/v1/chat/completions` | Exact | `llama-cpp`, `lm-studio`, `ollama`, `v-llm` |
| `openai/chat-completions-batch` | `POST` | `/v1/chat/completions/batch` | Exact | `v-llm` |
| `openai/chat-completions-control` | `POST` | `/v1/chat/completions/control` | Exact | `llama-cpp` |
| `openai/chat-completions-input-tokens` | `POST` | `/v1/chat/completions/input_tokens` | Exact | `llama-cpp` |
| `openai/responses` | `POST` | `/v1/responses` | Exact | `llama-cpp`, `lm-studio`, `ollama`, `v-llm` |
| `openai/responses-input-tokens` | `POST` | `/v1/responses/input_tokens` | Exact | `llama-cpp` |
| `openai/completions` | `POST` | `/v1/completions` | Exact | `llama-cpp`, `lm-studio`, `ollama`, `v-llm` |
| `openai/embeddings` | `POST` | `/v1/embeddings` | Exact | `llama-cpp`, `lm-studio`, `ollama`, `v-llm` |
| `openai/models` | `GET` | `^/v1/models(?:/[\w_-]+)?$` | Regex | `llama-cpp`, `lm-studio`, `ollama`, `v-llm` |
| `openai/audio-transcriptions` | `POST` | `/v1/audio/transcriptions` | Exact | `v-llm` |
| `openai/audio-translations` | `POST` | `/v1/audio/translations` | Exact | `v-llm` |

### Anthropic 互換 API

| API 名 (`name`) | メソッド | パス (`path`) | マッチ方式 | 対応プロバイダー |
|---|---|---|---|---|
| `anthropic/messages` | `POST` | `/v1/messages` | Exact | `llama-cpp`, `lm-studio`, `ollama`, `v-llm` |
| `anthropic/messages-count-tokens` | `POST` | `/v1/messages/count_tokens` | Exact | `llama-cpp`, `lm-studio`, `ollama`, `v-llm` |

---

## プロバイダー別対応 API 一覧

### llama-cpp
デフォルトのプロバイダー設定です。

- `openai/chat-completions` (`POST /v1/chat/completions`)
- `openai/chat-completions-control` (`POST /v1/chat/completions/control`)
- `openai/chat-completions-input-tokens` (`POST /v1/chat/completions/input_tokens`)
- `openai/responses` (`POST /v1/responses`)
- `openai/responses-input-tokens` (`POST /v1/responses/input_tokens`)
- `openai/completions` (`POST /v1/completions`)
- `openai/embeddings` (`POST /v1/embeddings`)
- `openai/models` (`GET ^/v1/models(?:/[\w_-]+)?$`)
- `anthropic/messages` (`POST /v1/messages`)
- `anthropic/messages-count-tokens` (`POST /v1/messages/count_tokens`)

### ollama

- `openai/chat-completions` (`POST /v1/chat/completions`)
- `openai/responses` (`POST /v1/responses`)
- `openai/completions` (`POST /v1/completions`)
- `openai/embeddings` (`POST /v1/embeddings`)
- `openai/models` (`GET ^/v1/models(?:/[\w_-]+)?$`)
- `anthropic/messages` (`POST /v1/messages`)
- `anthropic/messages-count-tokens` (`POST /v1/messages/count_tokens`)

### lm-studio

- `openai/chat-completions` (`POST /v1/chat/completions`)
- `openai/responses` (`POST /v1/responses`)
- `openai/completions` (`POST /v1/completions`)
- `openai/embeddings` (`POST /v1/embeddings`)
- `openai/models` (`GET ^/v1/models(?:/[\w_-]+)?$`)
- `anthropic/messages` (`POST /v1/messages`)
- `anthropic/messages-count-tokens` (`POST /v1/messages/count_tokens`)

### v-llm

- `openai/chat-completions` (`POST /v1/chat/completions`)
- `openai/chat-completions-batch` (`POST /v1/chat/completions/batch`)
- `openai/responses` (`POST /v1/responses`)
- `openai/completions` (`POST /v1/completions`)
- `openai/embeddings` (`POST /v1/embeddings`)
- `openai/models` (`GET ^/v1/models(?:/[\w_-]+)?$`)
- `openai/audio-transcriptions` (`POST /v1/audio/transcriptions`)
- `openai/audio-translations` (`POST /v1/audio/translations`)
- `anthropic/messages` (`POST /v1/messages`)
- `anthropic/messages-count-tokens` (`POST /v1/messages/count_tokens`)

---

## 設定方法

設定ファイル（`config.toml`）の `[api]` セクションで指定します。

### プロバイダーの一括指定

```toml
[api]
# "llama-cpp"（デフォルト）, "ollama", "lm-studio", "v-llm" から選択
provider = "v-llm"
```

### 個別 API の有効化 (named_apis)

特定のプロバイダープリセットに加えて追加の API を有効化したい場合や、プロバイダープリセットを使用せず個別の API のみ許可したい場合に指定します。

```toml
[api]
provider = "ollama"
# ollama プリセットには含まれない API を個別に追加
named_apis = [
  "openai/audio-transcriptions",
]
```

---

## CLI コマンドでの確認

CLI サブコマンド `predefined-apis` を使用して、定義済み API の一覧を表示できます。

```bash
# すべての定義済み API を表示
llm-auth-verifier predefined-apis

# 特定のプロバイダーでサポートされている API のみフィルタして表示
llm-auth-verifier predefined-apis --provider v-llm
llm-auth-verifier predefined-apis --provider llama-cpp
llm-auth-verifier predefined-apis --provider ollama
llm-auth-verifier predefined-apis --provider lm-studio
```
