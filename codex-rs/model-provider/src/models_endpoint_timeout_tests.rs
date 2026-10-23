use super::*;
use codex_http_client::OutboundProxyPolicy;
use codex_protocol::error::CodexErrorDetails;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test]
async fn catalog_deadline_returns_request_timeout() {
    let server = MockServer::start().await;
    let mock = Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "models": [] }))
                .set_delay(Duration::from_secs(/*secs*/ 60)),
        )
        .expect(1)
        .mount_as_scoped(&server)
        .await;
    let endpoint = OpenAiModelsEndpoint::new(
        ModelProviderInfo::create_openai_provider(Some(server.uri())),
        /*auth_manager*/ None,
        /*gateway_auth_manager*/ None,
    );

    tokio::time::pause();
    let request = tokio::time::timeout(
        MODELS_REFRESH_TIMEOUT * 2,
        endpoint.list_models(
            "0.0.0",
            HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        ),
    );
    tokio::pin!(request);
    let received = mock.wait_until_satisfied();
    tokio::pin!(received);
    let deadline = std::time::Instant::now() + Duration::from_secs(/*secs*/ 30);
    // Keep the paused runtime runnable until the request reaches the server.
    // Otherwise auto-advance can expire the catalog deadline during dispatch.
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "server did not receive the catalog request"
        );
        tokio::select! {
            () = &mut received => break,
            result = &mut request => {
                panic!("catalog request finished before reaching the server: {result:?}");
            }
            () = tokio::task::yield_now() => {}
        }
    }
    tokio::time::advance(MODELS_REFRESH_TIMEOUT + Duration::from_millis(/*millis*/ 1)).await;
    let error = request
        .await
        .expect("catalog deadline should expire")
        .expect_err("delayed catalog request should time out");
    tokio::time::resume();

    assert!(
        matches!(error.details(), CodexErrorDetails::RequestTimeout),
        "{error}"
    );
}

#[tokio::test]
async fn ollama_metadata_deadline_keeps_catalog_and_stops_scheduling_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": (0..6).map(|index| serde_json::json!({"id":format!("model-{index}")})).collect::<Vec<_>>()
        })))
        .mount(&server).await;
    let mock = Mock::given(method("POST"))
        .and(path("/api/show"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(
                    serde_json::json!({"thinking":{"values":[false,true],"default":true}}),
                )
                .set_delay(Duration::from_secs(60)),
        )
        .expect(MODEL_DETAILS_CONCURRENCY as u64)
        .mount_as_scoped(&server)
        .await;
    let endpoint = OpenAiModelsEndpoint::new(
        ModelProviderInfo {
            name: "Ollama Cloud".into(),
            base_url: Some(format!("{}/v1", server.uri())),
            model_catalog_format: ModelCatalogFormat::Openai,
            ..Default::default()
        },
        None,
        None,
    );
    tokio::time::pause();
    let request = endpoint.list_models(
        "version",
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
    );
    tokio::pin!(request);
    let received = mock.wait_until_satisfied();
    tokio::pin!(received);
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "metadata requests did not reach server"
        );
        tokio::select! {
            () = &mut received => break,
            result = &mut request => panic!("catalog returned before requests reached server: {result:?}"),
            () = tokio::task::yield_now() => {}
        }
    }
    tokio::time::advance(MODELS_REFRESH_TIMEOUT + Duration::from_millis(1)).await;
    let result = request.await.unwrap();
    tokio::time::resume();
    assert_eq!(result.models.len(), 6);
    assert!(
        result
            .models
            .iter()
            .all(|model| model.supported_reasoning_levels.is_empty())
    );
    assert_eq!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.method.as_str() == "POST")
            .count(),
        MODEL_DETAILS_CONCURRENCY
    );
}
