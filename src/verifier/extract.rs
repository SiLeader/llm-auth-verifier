use crate::api::{AiApi, ApiDetector};
use crate::config::ApiType;
use crate::verifier::Verifier;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, Method, Uri};
use std::str::FromStr;

impl Verifier {
    pub(super) fn extract_api<'a, 'b>(
        &'a self,
        headers: &'b HeaderMap,
    ) -> Option<(&'a AiApi, &'b str)> {
        extract_api_key(&self.api_detector, headers)
    }
}

fn check_api_type<'a>(detector: &'a ApiDetector, headers: &HeaderMap) -> Option<&'a AiApi> {
    let method = headers
        .get("x-forwarded-method")
        .and_then(|v| v.to_str().ok())?;
    let uri = headers
        .get("x-forwarded-uri")
        .and_then(|s| s.to_str().ok())?;

    let method = Method::from_str(method).ok()?;
    let uri: Uri = uri.parse().ok()?;
    detector.detect(&method, uri.path())
}

fn extract_api_key<'a, 'b>(
    detector: &'a ApiDetector,
    headers: &'b HeaderMap,
) -> Option<(&'a AiApi, &'b str)> {
    let api = check_api_type(detector, headers)?;
    // OpenAI / current Anthropic style:
    //
    // Authorization: Bearer xxx
    if let Some(value) = headers.get(AUTHORIZATION)
        && let Ok(value) = value.to_str()
        && let Some((scheme, token)) = value.split_once(" ")
    {
        let scheme = scheme.trim();
        let token = token.trim();
        if scheme.eq_ignore_ascii_case("bearer") {
            return Some((api, token));
        }
    }

    if api.api_type().is_some_and(|t| t == ApiType::Anthropic) {
        // Anthropic-compatible legacy style:
        //
        // x-api-key: xxx
        headers
            .get("x-api-key")
            .and_then(|value| value.to_str().ok())
            .map(|t| (api, t))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::extract_api_key;
    use crate::api::{ApiConfig, ApiDetector, Provider};
    use crate::config::ApiType;
    use axum::http::HeaderMap;

    fn headers(method: &'static str, uri: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-method", method.parse().unwrap());
        headers.insert("x-forwarded-uri", uri.parse().unwrap());
        headers
    }

    fn headers_with_authorization(value: &'static str) -> HeaderMap {
        let mut headers = headers("POST", "/v1/chat/completions");
        headers.insert("authorization", value.parse().unwrap());
        headers
    }

    fn detector() -> ApiDetector {
        ApiDetector::try_new(ApiConfig::for_provider(Provider::LlamaCpp)).unwrap()
    }

    fn assert_extracted(
        result: Option<(&crate::api::AiApi, &str)>,
        expected_name: &str,
        expected_type: ApiType,
        expected_token: &str,
    ) {
        let (api, token) = result.unwrap();
        assert_eq!(api.name(), expected_name);
        assert_eq!(api.api_type(), Some(expected_type));
        assert_eq!(token, expected_token);
    }

    #[test]
    fn accepts_case_insensitive_bearer_scheme() {
        let detector = detector();
        let headers = headers_with_authorization("bearer secret");

        assert_extracted(
            extract_api_key(&detector, &headers),
            "openai/chat-completions",
            ApiType::OpenAi,
            "secret",
        );
    }

    #[test]
    fn accepts_multiple_spaces_after_bearer_scheme() {
        let detector = detector();
        let headers = headers_with_authorization("BEARER   secret");

        assert_extracted(
            extract_api_key(&detector, &headers),
            "openai/chat-completions",
            ApiType::OpenAi,
            "secret",
        );
    }

    #[test]
    fn rejects_non_bearer_authorization_scheme() {
        let detector = detector();
        let headers = headers_with_authorization("Basic secret");

        assert!(extract_api_key(&detector, &headers).is_none());
    }

    #[test]
    fn accepts_anthropic_api_key_header() {
        let detector = detector();
        let mut headers = headers("POST", "/v1/messages");
        headers.insert("x-api-key", "secret".parse().unwrap());

        assert_extracted(
            extract_api_key(&detector, &headers),
            "anthropic/messages",
            ApiType::Anthropic,
            "secret",
        );
    }

    #[test]
    fn rejects_api_key_header_for_openai_api() {
        let detector = detector();
        let mut headers = headers("POST", "/v1/chat/completions");
        headers.insert("x-api-key", "secret".parse().unwrap());

        assert!(extract_api_key(&detector, &headers).is_none());
    }

    #[test]
    fn uses_uri_path_without_query_when_detecting_api() {
        let detector = detector();
        let mut headers = headers("POST", "/v1/chat/completions?stream=true");
        headers.insert("authorization", "Bearer secret".parse().unwrap());

        assert_extracted(
            extract_api_key(&detector, &headers),
            "openai/chat-completions",
            ApiType::OpenAi,
            "secret",
        );
    }

    #[test]
    fn rejects_missing_or_non_matching_forwarded_request() {
        let detector = detector();
        let mut missing_method = HeaderMap::new();
        missing_method.insert("x-forwarded-uri", "/v1/chat/completions".parse().unwrap());
        missing_method.insert("authorization", "Bearer secret".parse().unwrap());
        let mut wrong_method = headers("GET", "/v1/chat/completions");
        wrong_method.insert("authorization", "Bearer secret".parse().unwrap());

        assert!(extract_api_key(&detector, &missing_method).is_none());
        assert!(extract_api_key(&detector, &wrong_method).is_none());
    }
}
