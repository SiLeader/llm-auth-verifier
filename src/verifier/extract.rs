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
    use super::{ApiType, extract_api_key};
    use crate::api::{ApiConfig, ApiDetector, Provider};
    use axum::http::HeaderMap;

    fn headers_with_authorization(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-uri", "/v1/chat/completions".parse().unwrap());
        headers.insert("authorization", value.parse().unwrap());
        headers
    }

    fn detector() -> ApiDetector {
        ApiDetector::try_new(ApiConfig::for_provider(Provider::LlamaCpp)).unwrap()
    }

    #[test]
    fn accepts_case_insensitive_bearer_scheme() {
        let detector = detector();
        let headers = headers_with_authorization("bearer secret");

        assert_eq!(
            extract_api_key(&detector, &headers),
            Some((ApiType::OpenAi, "secret"))
        );
    }

    #[test]
    fn accepts_multiple_spaces_after_bearer_scheme() {
        let detector = detector();
        let headers = headers_with_authorization("BEARER   secret");

        assert_eq!(
            extract_api_key(&detector, &headers),
            Some((ApiType::OpenAi, "secret"))
        );
    }

    #[test]
    fn rejects_non_bearer_authorization_scheme() {
        let detector = detector();
        let headers = headers_with_authorization("Basic secret");

        assert_eq!(extract_api_key(&detector, &headers), None);
    }
}
