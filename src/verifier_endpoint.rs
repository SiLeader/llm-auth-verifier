use crate::verifier::Verifier;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};

pub(crate) async fn verify_auth(
    State(verifier): State<Verifier>,
    headers: HeaderMap,
) -> StatusCode {
    if verifier.verify_token(&headers).await {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    }
}
