use crate::config::ApiType;
use crate::verifier::Verifier;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode, Uri};
use std::collections::HashSet;
use std::sync::LazyLock;

pub(crate) async fn verify_auth(
    State(verifier): State<Verifier>,
    headers: HeaderMap,
) -> StatusCode {
    let Some((api_type, api_key)) = extract_api_key(&headers) else {
        return StatusCode::UNAUTHORIZED;
    };

    if verifier.verify_token(api_type, api_key) {
        return StatusCode::OK;
    }

    StatusCode::UNAUTHORIZED
}

static ANTHROPIC_API_ENDPOINT: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from_iter(["/v1/messages"]));
static OPENAI_API_ENDPOINT: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from_iter([
        "/v1/models",
        "/v1/responses",
        "/v1/chat/completions",
        "/v1/embeddings",
        "/v1/completions",
    ])
});

fn check_api_type(headers: &HeaderMap) -> Option<ApiType> {
    if let Some(uri) = headers.get("x-forwarded-uri").and_then(|s| s.to_str().ok()) {
        let uri: Uri = uri.parse().ok()?;
        if ANTHROPIC_API_ENDPOINT.contains(uri.path()) {
            Some(ApiType::Anthropic)
        } else if OPENAI_API_ENDPOINT.contains(uri.path()) {
            Some(ApiType::OpenAi)
        } else {
            None
        }
    } else {
        None
    }
}

fn extract_api_key(headers: &HeaderMap) -> Option<(ApiType, &str)> {
    let api_type = check_api_type(headers)?;
    // OpenAI / current Anthropic style:
    //
    // Authorization: Bearer xxx
    if let Some(value) = headers.get(AUTHORIZATION)
        && let Ok(value) = value.to_str()
        && let Some((scheme, token)) = value.split_once(" ")
    {
        let scheme = scheme.trim();
        let token = token.trim();
        if scheme.eq_ignore_ascii_case("Bearer") {
            return Some((api_type, token));
        }
    }

    if api_type == ApiType::Anthropic {
        // Anthropic-compatible legacy style:
        //
        // x-api-key: xxx
        headers
            .get("x-api-key")
            .and_then(|value| value.to_str().ok())
            .map(|t| (api_type, t))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiType, extract_api_key};
    use axum::http::HeaderMap;

    fn headers_with_authorization(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-uri", "/v1/chat/completions".parse().unwrap());
        headers.insert("authorization", value.parse().unwrap());
        headers
    }

    #[test]
    fn accepts_case_insensitive_bearer_scheme() {
        let headers = headers_with_authorization("bearer secret");

        assert_eq!(extract_api_key(&headers), Some((ApiType::OpenAi, "secret")));
    }

    #[test]
    fn accepts_multiple_spaces_after_bearer_scheme() {
        let headers = headers_with_authorization("BEARER   secret");

        assert_eq!(extract_api_key(&headers), Some((ApiType::OpenAi, "secret")));
    }

    #[test]
    fn rejects_non_bearer_authorization_scheme() {
        let headers = headers_with_authorization("Basic secret");

        assert_eq!(extract_api_key(&headers), None);
    }
}
