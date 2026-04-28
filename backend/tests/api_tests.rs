use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use backend::api::handlers::profiling::get_health;

#[tokio::test]
async fn test_health_check_integration() {
    let response = get_health().await.expect("Health check failed");
    let response = response.into_response();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_stellar_toml_headers() {
    use backend::api::handlers::stellar::get_stellar_toml;
    let response = get_stellar_toml().await.into_response();
    
    assert_eq!(response.status(), StatusCode::OK);
    let cors = response.headers().get("access-control-allow-origin").unwrap();
    assert_eq!(cors, "*");
}
