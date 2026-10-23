use super::*;
use codex_http_client::OutboundProxyPolicy;
use codex_login::{AuthManager, CodexAuth};
use codex_model_provider::create_model_provider;
use codex_model_provider_info::ModelCatalogFormat;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn live_settings() -> SearchSettings {
    SearchSettings {
        external_web_access: Some(codex_api::ExternalWebAccess::Boolean(true)),
        ..Default::default()
    }
}

fn factory() -> HttpClientFactory {
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
}

fn provider(name: &str, base: String) -> SharedModelProvider {
    create_model_provider(
        ModelProviderInfo {
            name: name.into(),
            base_url: Some(base),
            model_catalog_format: ModelCatalogFormat::Openai,
            experimental_bearer_token: Some("native-web-key".into()),
            ..Default::default()
        },
        Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
            "unrelated-chatgpt-key",
        ))),
    )
}

#[tokio::test]
async fn native_web_search_and_fetch_follow_provider_contract_and_reuse_pages() {
    for (name, base, search, fetch, search_body) in [
        (
            "Ollama Cloud",
            "/proxy/v1",
            "/proxy/api/web_search",
            "/proxy/api/web_fetch",
            json!({"query":"rust docs","max_results":3}),
        ),
        (
            "DeepSeek",
            "/proxy",
            "/proxy/v1/tools/search",
            "/proxy/v1/tools/fetch",
            json!({"query":"rust docs","count":3,"need_content":false}),
        ),
        (
            "DeepSeek",
            "/proxy/v1",
            "/proxy/v1/tools/search",
            "/proxy/v1/tools/fetch",
            json!({"query":"rust docs","count":3,"need_content":false}),
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path(search)).and(header("authorization", "Bearer native-web-key"))
            .and(body_json(search_body)).respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[{"title":"Rust","url":"https://example.com/docs","summary":"Search summary","content":"Ollama snippet"}]}))).expect(1).mount(&server).await;
        Mock::given(method("POST")).and(path(fetch)).and(header("authorization", "Bearer native-web-key"))
            .and(body_json(json!({"url":"https://example.com/docs"}))).respond_with(ResponseTemplate::new(200).set_body_json(json!({"title":"Rust docs","content":"one\ntwo\nneedle\nfour","links":["https://example.com/next"]}))).expect(1).mount(&server).await;
        let provider = provider(name, format!("{}{base}", server.uri()));
        let state = PublicWebState::default();
        let response=state.run(r#"{"search_query":[{"q":"rust docs","max_results":3}],"open":[{"ref_id":"https://example.com/docs","lineno":1,"num_lines":2}]}"#, &provider, &factory(), &live_settings()).await.unwrap();
        let output: Value = serde_json::from_str(&response.output).unwrap();
        assert_eq!(
            output["documents"][0]["results"][0]["url"],
            "https://example.com/docs"
        );
        assert_eq!(
            output["documents"][1]["lines"],
            json!([{"line":1,"text":"two","line_truncated":false},{"line":2,"text":"needle","line_truncated":false}])
        );
        assert_eq!(response.results.unwrap()[0]["type"], "text_result");
        assert_eq!(
            output["documents"][1]["links"],
            json!(["https://example.com/next"])
        );
        let response = state
            .run(
                r#"{"find":[{"ref_id":"https://example.com/docs","pattern":"needle"}]}"#,
                &provider,
                &factory(),
                &live_settings(),
            )
            .await
            .unwrap();
        assert!(response.output.contains("needle"));
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2, "find must reuse the fetched page");
        for request in requests {
            assert!(request.headers.get("x-codex-turn-metadata").is_none());
            assert!(request.headers.get("chatgpt-account-id").is_none());
        }
    }
}

#[tokio::test]
async fn native_web_rejects_invalid_or_unsupported_operations_before_network() {
    let server = MockServer::start().await;
    let provider = provider("DeepSeek", server.uri());
    let state = PublicWebState::default();
    for arguments in [
        r#"{}"#,
        r#"{"screenshot":[{"ref_id":"https://example.com","pageno":0}]}"#,
        r#"{"open":[{"ref_id":"file:///etc/passwd"}]}"#,
        r#"{"open":[{"ref_id":"https://user:password@example.com/"}]}"#,
        r#"{"open":[{"ref_id":"https://example.com/","num_lines":121}]}"#,
        r#"{"search_query":[{"q":" "}]}"#,
        r#"{"search_query":[{"q":"valid","domains":["example.com"]}]}"#,
        r#"{"search_query":[{"q":"valid","max_results":11}]}"#,
        r#"{"search_query":[{"q":"a"},{"q":"b"},{"q":"c"},{"q":"d"},{"q":"e"}]}"#,
    ] {
        assert!(matches!(
            state
                .run(arguments, &provider, &factory(), &live_settings())
                .await,
            Err(FunctionCallError::RespondToModel(_))
        ));
    }
    let settings = SearchSettings {
        filters: Some(codex_api::SearchFilters {
            allowed_domains: Some(vec!["example.com".into()]),
            ..Default::default()
        }),
        ..live_settings()
    };
    assert!(
        state
            .run(
                r#"{"search_query":[{"q":"valid"}]}"#,
                &provider,
                &factory(),
                &settings
            )
            .await
            .is_err()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(
        state
            .run(
                r#"{"search_query":[{"q":"valid"}]}"#,
                &provider,
                &factory(),
                &SearchSettings::default()
            )
            .await
            .is_err()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn native_web_hanging_service_has_a_bounded_deadline() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(45))
                .set_body_json(json!({"results":[]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = provider("DeepSeek", server.uri());
    let started = std::time::Instant::now();
    let error = PublicWebState::default()
        .run(
            r#"{"search_query":[{"q":"hello"}]}"#,
            &provider,
            &factory(),
            &live_settings(),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(40));
}

#[tokio::test]
async fn native_web_errors_are_sanitized_and_not_retried_or_redirected() {
    let sink = MockServer::start().await;
    for code in [302, 401, 402, 403, 404, 429, 500] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/web_search"))
            .respond_with(
                ResponseTemplate::new(code)
                    .insert_header("Location", sink.uri())
                    .set_body_string("secret-provider-body"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = provider("Ollama Cloud", format!("{}/v1", server.uri()));
        let error = PublicWebState::default()
            .run(
                r#"{"search_query":[{"q":"hello"}]}"#,
                &provider,
                &factory(),
                &live_settings(),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, FunctionCallError::RespondToModel(_)));
        assert!(!error.to_string().contains("secret-provider-body"));
        assert!(!error.to_string().contains("native-web-key"));
    }
    assert!(sink.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn native_web_bounds_response_sizes_and_rejects_bad_json() {
    for body in ["bad json".into(), "x".repeat(MAX_RESPONSE_BYTES + 1)] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let provider = provider("DeepSeek", server.uri());
        assert!(
            PublicWebState::default()
                .run(
                    r#"{"search_query":[{"q":"hello"}]}"#,
                    &provider,
                    &factory(),
                    &live_settings()
                )
                .await
                .is_err()
        );
    }
}

#[test]
fn native_web_schema_hides_unsupported_hosted_commands_and_preserves_endpoint_queries() {
    let properties = schema()["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(properties, vec!["find", "open", "search_query"]);
    for (name, url, expected) in [
        (
            "Ollama Cloud",
            "https://example.com/prefix/v1/models?tenant=a",
            "https://example.com/prefix/api/web_search?tenant=a",
        ),
        (
            "DeepSeek",
            "https://example.com/prefix/v1/models?tenant=a",
            "https://example.com/prefix/v1/tools/search?tenant=a",
        ),
    ] {
        let provider = ModelProviderInfo {
            name: name.into(),
            ..Default::default()
        };
        assert_eq!(endpoint(&provider, url, false).unwrap(), expected);
    }
    let page = Page {
        url: "https://example.com".into(),
        title: "title".into(),
        content: "x".repeat(2000),
        truncated: false,
        links: Vec::new(),
    };
    assert_eq!(page_window(&page, 0, 1)["lines"][0]["line_truncated"], true);
    assert!(
        page_window(&page, usize::MAX, 1)["lines"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
