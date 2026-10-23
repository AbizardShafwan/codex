//! Read-only provider-native balances and quotas. Never uses ambient ChatGPT credentials.

use std::sync::Arc;
use std::time::Duration;

use codex_api::{ModelsClient, ReqwestTransport};
use codex_http_client::{ClientRouteClass, HttpClientFactory};
use codex_login::AuthManager;
use codex_login::default_client::{ClientRedirectPolicy, create_client_for_route_async};
use codex_model_provider_info::{ModelCatalogFormat, ModelProviderInfo};
use codex_protocol::account::{ProviderAccountStatus, ProviderBalance, ProviderUsageWindow};
use codex_protocol::error::{CodexErr, Result};
use serde_json::Value;

fn unavailable() -> CodexErr {
    CodexErr::InvalidRequest(
        "Provider account data unavailable. Check the provider connection and retry later.".into(),
    )
}

fn account_url(name: &str, models_url: &str) -> Option<String> {
    let mut url = url::Url::parse(models_url).ok()?;
    let path = match name {
        "Ollama Cloud" => format!("{}/api/balance", url.path().strip_suffix("/v1/models")?),
        "DeepSeek" => {
            let prefix = url.path().strip_suffix("/models")?;
            format!(
                "{}/user/balance",
                prefix.strip_suffix("/v1").unwrap_or(prefix)
            )
        }
        _ => return None,
    };
    url.set_path(&path);
    Some(url.into())
}

pub async fn read_provider_account_status(
    mut provider: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
    http_client_factory: HttpClientFactory,
) -> Result<ProviderAccountStatus> {
    if provider.model_catalog_format != ModelCatalogFormat::Openai {
        return Err(unavailable());
    }
    crate::provider::load_saved_provider_key(&mut provider, auth_manager.as_deref())
        .map_err(|_| unavailable())?;
    // Account endpoints are demand-driven. Do not automatically retry 429s or poll them.
    provider.request_max_retries = Some(0);
    let api_provider = provider.to_api_provider(None)?;
    let request_url = account_url(
        &provider.name,
        &api_provider.build_request(http::Method::GET, "models").url,
    )
    .ok_or_else(unavailable)?;
    let auth = crate::auth::resolve_provider_auth(None, &provider)?;
    let read = async {
        let http = create_client_for_route_async(
            http_client_factory,
            request_url.clone(),
            ClientRouteClass::Api,
            ClientRedirectPolicy::Reject,
        )
        .await
        .map_err(|_| unavailable())?
        .without_request_logging();
        let client =
            ModelsClient::new(ReqwestTransport::from_http_client(http), api_provider, auth);
        let body = client
            .account_status_raw(request_url, 1024 * 1024)
            .await
            .map_err(|_| unavailable())?;
        parse_status(&provider.name, &body)
    };
    tokio::time::timeout(Duration::from_secs(10), read)
        .await
        .map_err(|_| unavailable())?
}

fn decimal(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => return None,
    };
    // Preserve reported decimal precision without accepting terminal control sequences.
    let number = text.parse::<f64>().ok()?;
    (number.is_finite()
        && !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E')))
    .then_some(text)
}

fn reset(value: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|date| date.timestamp())
}

fn parse_status(name: &str, body: &[u8]) -> Result<ProviderAccountStatus> {
    let value: Value = serde_json::from_slice(body).map_err(|_| unavailable())?;
    let mut status = ProviderAccountStatus {
        balances: Vec::new(),
        limits: Vec::new(),
    };
    match name {
        "DeepSeek" => {
            for info in value["balance_infos"].as_array().ok_or_else(unavailable)? {
                let currency = info["currency"]
                    .as_str()
                    .filter(|value| matches!(*value, "USD" | "CNY"))
                    .ok_or_else(unavailable)?;
                for (field, label) in [
                    ("total_balance", "Balance"),
                    ("granted_balance", "Granted"),
                    ("topped_up_balance", "Topped up"),
                ] {
                    status.balances.push(ProviderBalance {
                        label: label.into(),
                        currency: currency.into(),
                        amount: decimal(&info[field]).ok_or_else(unavailable)?,
                        resets_at: None,
                    });
                }
            }
        }
        "Ollama Cloud" => {
            for (field, label) in [("session", "5h limit"), ("weekly", "Weekly limit")] {
                if let Some(window) = value["included"].get(field) {
                    let remaining = window["remaining_percent"]
                        .as_f64()
                        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
                        .ok_or_else(unavailable)?;
                    status.limits.push(ProviderUsageWindow {
                        label: label.into(),
                        remaining_percent: remaining,
                        resets_at: reset(&window["resets_at"]),
                    });
                }
            }
            for (field, label) in [
                ("included", "Included credits"),
                ("purchased", "Purchased credits"),
            ] {
                if let Some(amount) = value[field].get("balance_usd") {
                    status.balances.push(ProviderBalance {
                        label: label.into(),
                        currency: "USD".into(),
                        amount: decimal(amount).ok_or_else(unavailable)?,
                        resets_at: reset(&value[field]["period"]["until"]),
                    });
                }
            }
        }
        _ => return Err(unavailable()),
    }
    if status.balances.is_empty() && status.limits.is_empty() {
        return Err(unavailable());
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_http_client::OutboundProxyPolicy;
    use codex_login::CodexAuth;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn provider_status_preserves_decimal_balances_and_native_reset_windows() {
        let deepseek = json!({"is_available":true,"balance_infos":[{"currency":"USD","total_balance":"110.000001","granted_balance":"10.000001","topped_up_balance":"100.00"}]});
        let status = parse_status("DeepSeek", &serde_json::to_vec(&deepseek).unwrap()).unwrap();
        assert_eq!(status.balances[0].amount, "110.000001");
        assert_eq!(status.balances[0].currency, "USD");
        assert_eq!(status.balances.len(), 3);
        assert!(status.limits.is_empty());
        let ollama = json!({"included":{"session":{"remaining_percent":75,"resets_at":"2026-10-01T07:00:00Z"},"weekly":{"remaining_percent":40,"resets_at":"2026-10-05T00:00:00Z"}},"purchased":{"balance_usd":25}});
        let status = parse_status("Ollama Cloud", &serde_json::to_vec(&ollama).unwrap()).unwrap();
        assert_eq!(status.limits[0].label, "5h limit");
        assert_eq!(status.limits[0].remaining_percent, 75.0);
        assert_eq!(status.limits[1].remaining_percent, 40.0);
        assert!(
            status
                .limits
                .iter()
                .all(|window| window.resets_at.is_some())
        );
        assert_eq!(status.balances[0].amount, "25");
        let monthly = json!({"included":{"balance_usd":72.5,"allowance_usd":100,"period":{"until":"2026-10-15T09:30:00Z"}},"purchased":{"balance_usd":25}});
        let status = parse_status("Ollama Cloud", &serde_json::to_vec(&monthly).unwrap()).unwrap();
        assert!(
            status.limits.is_empty(),
            "monthly plans must not invent rolling quotas"
        );
        assert_eq!(status.balances[0].amount, "72.5");
        assert!(status.balances[0].resets_at.is_some());
        assert!(
            parse_status(
                "Ollama Cloud",
                br#"{"included":{"session":{"remaining_percent":101}}}"#
            )
            .is_err()
        );
        assert!(
            parse_status(
                "DeepSeek",
                br#"{"balance_infos":[{"currency":"USD","total_balance":"bad\nvalue"}]}"#
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn provider_status_uses_only_provider_credentials_and_expected_endpoint() {
        for (name, expected_path, body) in [
            (
                "DeepSeek",
                "/user/balance",
                json!({"balance_infos":[{"currency":"CNY","total_balance":"110.00","granted_balance":"10.00","topped_up_balance":"100.00"}]}),
            ),
            (
                "Ollama Cloud",
                "/api/balance",
                json!({"included":{"session":{"remaining_percent":75,"resets_at":"2026-10-01T07:00:00Z"}},"purchased":{"balance_usd":25}}),
            ),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path(expected_path))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            let status = read_provider_account_status(
                ModelProviderInfo {
                    name: name.into(),
                    base_url: Some(format!("{}/v1", server.uri())),
                    experimental_bearer_token: Some("provider-only-key".into()),
                    model_catalog_format: ModelCatalogFormat::Openai,
                    ..Default::default()
                },
                Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
                    "unrelated-chatgpt-key",
                ))),
                HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            )
            .await
            .unwrap();
            assert!(!status.balances.is_empty());
            let requests = server.received_requests().await.unwrap();
            assert_eq!(
                requests[0].headers["authorization"],
                "Bearer provider-only-key"
            );
            assert!(!requests[0].headers.contains_key("chatgpt-account-id"));
        }
    }

    #[tokio::test]
    async fn provider_status_rejects_redirects_and_does_not_retry_rate_limits() {
        let destination = MockServer::start().await;
        for code in [302, 429] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/api/balance"))
                .respond_with(
                    ResponseTemplate::new(code)
                        .insert_header("location", format!("{}/api/balance", destination.uri()))
                        .insert_header("retry-after", "60")
                        .set_body_string("provider-secret"),
                )
                .expect(1)
                .mount(&server)
                .await;
            let error = read_provider_account_status(
                ModelProviderInfo {
                    name: "Ollama Cloud".into(),
                    base_url: Some(format!("{}/v1", server.uri())),
                    experimental_bearer_token: Some("provider-only-key".into()),
                    model_catalog_format: ModelCatalogFormat::Openai,
                    ..Default::default()
                },
                None,
                HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            )
            .await
            .unwrap_err();
            assert!(!error.to_string().contains("provider-secret"));
        }
        assert!(destination.received_requests().await.unwrap().is_empty());
    }
}
