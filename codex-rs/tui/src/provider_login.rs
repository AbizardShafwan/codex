//! Provider selection uses the server's credentials and catalogs, never the client's environment.

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ModelListParams;
use codex_app_server_protocol::ModelListResponse;
use codex_app_server_protocol::RequestId;
use codex_protocol::openai_models::ModelPreset;
use color_eyre::eyre::Result;

pub(crate) struct ProviderSelection {
    pub provider_id: String,
    pub model: ModelPreset,
    pub models: Vec<ModelPreset>,
}

pub(crate) async fn select_provider(
    request_handle: AppServerRequestHandle,
    provider_id: String,
    preferred_model: Option<&str>,
) -> Result<ProviderSelection> {
    let mut models = Vec::new();
    let mut cursor = None;
    loop {
        let response = request_handle
            .request_typed::<ModelListResponse>(ClientRequest::ModelList {
                request_id: RequestId::String(format!("provider-models-{}", uuid::Uuid::new_v4())),
                params: ModelListParams {
                    model_provider: Some(provider_id.clone()),
                    cursor,
                    limit: None,
                    include_hidden: Some(false),
                },
            })
            .await?;
        models.extend(
            response
                .data
                .into_iter()
                .map(crate::app_server_session::model_preset_from_api_model),
        );
        cursor = response.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    let model = models
        .iter()
        .find(|model| Some(model.model.as_str()) == preferred_model)
        .or_else(|| models.iter().find(|model| model.is_default))
        .or_else(|| models.first())
        .cloned()
        .ok_or_else(|| {
            color_eyre::eyre::eyre!(
                "No models are available for this provider. Check its credentials and connection."
            )
        })?;
    Ok(ProviderSelection {
        provider_id,
        model,
        models,
    })
}

pub(crate) async fn persist_provider_selection(
    request_handle: AppServerRequestHandle,
    selection: &ProviderSelection,
) -> Result<()> {
    let effort = (!selection.model.supported_reasoning_efforts.is_empty())
        .then_some(&selection.model.default_reasoning_effort);
    let mut edits =
        crate::config_update::build_model_selection_edits(&selection.model.model, effort);
    edits.push(crate::config_update::replace_config_value(
        "model_provider",
        serde_json::json!(selection.provider_id),
    ));
    edits.extend(crate::config_update::build_service_tier_selection_edits(
        None,
    ));
    crate::config_update::write_config_batch(request_handle, edits).await?;
    Ok(())
}
