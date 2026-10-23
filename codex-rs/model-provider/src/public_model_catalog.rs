//! Decode public model IDs without importing provider-authored behavior instructions.

use std::collections::HashSet;

use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::model_info::model_info_from_slug;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::ToolMode;
use serde::Deserialize;

#[derive(Deserialize)]
struct PublicModelsResponse {
    data: Vec<PublicModel>,
}

#[derive(Deserialize)]
struct PublicModel {
    id: String,
    #[serde(default)]
    effort: Option<serde_json::Value>,
}

fn named_effort(value: &str) -> Option<ReasoningEffort> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return None;
    }
    value.parse().ok()
}

fn apply_deepseek_thinking(model: &mut ModelInfo, value: serde_json::Value) {
    #[derive(Deserialize)]
    struct Effort {
        supported_levels: Vec<String>,
        default_level: Option<String>,
    }
    let Ok(metadata) = serde_json::from_value::<Effort>(value) else {
        return;
    };
    let Some(levels) = metadata
        .supported_levels
        .iter()
        .map(|level| named_effort(level))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    // If the server omits its default, use the first recommended choice in the picker.
    let default = match metadata.default_level.as_deref() {
        Some(value) => named_effort(value),
        None => levels.first().cloned(),
    };
    let Some(default) = default else {
        return;
    };
    if !levels.contains(&default) {
        return;
    }
    let mut seen = HashSet::new();
    model.supported_reasoning_levels = std::iter::once(ReasoningEffort::None)
        .chain(levels)
        .filter(|effort| seen.insert(effort.clone()))
        .map(|effort| ReasoningEffortPreset {
            description: if effort == ReasoningEffort::None {
                "Thinking off".to_string()
            } else {
                format!("{} thinking effort", effort.as_str())
            },
            effort,
        })
        .collect();
    model.default_reasoning_level = Some(default);
}

/// Read thinking controls only: provider templates and system prompts are never imported.
pub(crate) fn apply_ollama_thinking(model: &mut ModelInfo, body: &[u8]) {
    #[derive(Deserialize)]
    struct Details {
        thinking: Thinking,
    }
    #[derive(Deserialize)]
    struct Thinking {
        values: Vec<serde_json::Value>,
        default: serde_json::Value,
    }
    fn effort(value: &serde_json::Value) -> Option<ReasoningEffort> {
        match value {
            serde_json::Value::Bool(false) => Some(ReasoningEffort::None),
            // Ollama's Responses compatibility maps high to true on on/off-only models.
            serde_json::Value::Bool(true) => Some(ReasoningEffort::High),
            serde_json::Value::String(value) => named_effort(value),
            _ => None,
        }
    }
    let Ok(details) = serde_json::from_slice::<Details>(body) else {
        return;
    };
    let Some(default) = effort(&details.thinking.default) else {
        return;
    };
    let Some(values) = details
        .thinking
        .values
        .iter()
        .map(effort)
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    if !values.contains(&default) {
        return;
    }
    let mut seen = HashSet::new();
    model.supported_reasoning_levels = values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .map(|effort| ReasoningEffortPreset {
            description: match effort {
                ReasoningEffort::None => "Thinking off".to_string(),
                ReasoningEffort::High
                    if details
                        .thinking
                        .values
                        .contains(&serde_json::Value::Bool(true)) =>
                {
                    "Thinking on".to_string()
                }
                _ => format!("{} thinking effort", effort.as_str()),
            },
            effort,
        })
        .collect();
    model.default_reasoning_level = Some(default);
}

pub(crate) fn parse_public_models(
    body: &[u8],
    provider: &ModelProviderInfo,
) -> Result<Vec<ModelInfo>> {
    let response: PublicModelsResponse = serde_json::from_slice(body).map_err(|_| {
        CodexErr::InvalidRequest("invalid public model catalog response".to_string())
    })?;
    let mut seen = HashSet::new();
    response
        .data
        .into_iter()
        .map(|entry| {
            if entry.id.trim().is_empty()
                || entry.id != entry.id.trim()
                || entry.id.chars().any(char::is_control)
                || !seen.insert(entry.id.clone())
            {
                return Err(CodexErr::InvalidRequest(
                    "model catalog must contain unique, non-empty model IDs".to_string(),
                ));
            }
            let mut model = model_info_from_slug(&entry.id);
            model.visibility = ModelVisibility::List;
            // Bare public IDs do not establish capabilities. Start conservatively, then
            // apply only recognized provider metadata, never unrelated OpenAI defaults.
            model.context_window = None;
            model.max_context_window = None;
            model.input_modalities = vec![InputModality::Text];
            model.supports_reasoning_summary_parameter = false;
            model.default_reasoning_summary = ReasoningSummary::None;
            model.tool_mode = Some(ToolMode::Direct);
            model.used_fallback_model_metadata = false;
            if provider.name == "DeepSeek"
                && matches!(model.slug.as_str(), "deepseek-flash" | "deepseek-v4-pro")
            {
                // Advisory capabilities published by DeepSeek's official harness and API docs.
                // Unknown models remain conservative instead of inheriting these by prefix.
                model.context_window = Some(1_000_000);
                model.max_context_window = Some(1_000_000);
                model.default_reasoning_level = Some(ReasoningEffort::High);
                model.supported_reasoning_levels = [
                    ReasoningEffort::None,
                    ReasoningEffort::Low,
                    ReasoningEffort::High,
                    ReasoningEffort::Max,
                ]
                .into_iter()
                .map(|effort| ReasoningEffortPreset {
                    description: format!("{} reasoning effort", effort.as_str()),
                    effort,
                })
                .collect();
                model.apply_patch_tool_type = Some(ApplyPatchToolType::Freeform);
                if model.slug == "deepseek-flash" {
                    model.input_modalities = vec![InputModality::Text, InputModality::Image];
                    model.supports_image_detail_original = true;
                }
            }
            if provider.name == "DeepSeek"
                && let Some(effort) = entry.effort
            {
                apply_deepseek_thinking(&mut model, effort);
            }
            Ok(model)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(model: &ModelInfo) -> Vec<ReasoningEffort> {
        model
            .supported_reasoning_levels
            .iter()
            .map(|preset| preset.effort.clone())
            .collect()
    }

    #[test]
    fn ollama_advertised_thinking_controls_preserve_defaults_and_custom_levels() {
        for (body, expected, default) in [
            (
                serde_json::json!({"values":[false,"low","high","max"],"default":"high"}),
                vec![
                    ReasoningEffort::None,
                    ReasoningEffort::Low,
                    ReasoningEffort::High,
                    ReasoningEffort::Max,
                ],
                ReasoningEffort::High,
            ),
            (
                serde_json::json!({"values":[false,true],"default":true}),
                vec![ReasoningEffort::None, ReasoningEffort::High],
                ReasoningEffort::High,
            ),
            (
                serde_json::json!({"values":[false],"default":false}),
                vec![ReasoningEffort::None],
                ReasoningEffort::None,
            ),
            (
                serde_json::json!({"values":["balanced","thorough","balanced"],"default":"balanced"}),
                vec![
                    ReasoningEffort::Custom("balanced".into()),
                    ReasoningEffort::Custom("thorough".into()),
                ],
                ReasoningEffort::Custom("balanced".into()),
            ),
        ] {
            let mut model = model_info_from_slug("native-model");
            let messages = model.model_messages.clone();
            let body = serde_json::to_vec(&serde_json::json!({"thinking":body,"system":"replace policy","template":"replace policy"})).unwrap();
            apply_ollama_thinking(&mut model, &body);
            assert_eq!(levels(&model), expected);
            assert_eq!(model.default_reasoning_level, Some(default));
            assert_eq!(model.model_messages, messages);
        }
    }

    #[test]
    fn invalid_or_absent_ollama_thinking_metadata_leaves_capabilities_unchanged() {
        for body in [
            "{}",
            r#"{"thinking":null}"#,
            r#"{"thinking":{"values":[],"default":"high"}}"#,
            r#"{"thinking":{"values":[1],"default":1}}"#,
            r#"{"thinking":{"values":["high"],"default":"low"}}"#,
            r#"{"thinking":{"values":[" high"],"default":" high"}}"#,
            "not json",
        ] {
            let mut model = model_info_from_slug("unknown");
            let original = model.clone();
            apply_ollama_thinking(&mut model, body.as_bytes());
            assert_eq!(model, original);
        }
    }

    #[test]
    fn deepseek_catalog_uses_native_effort_metadata_for_new_model_ids() {
        let body = br#"{"data":[{"id":"deepseek-new-alias","effort":{"supported_levels":["low","high","max"],"default_level":"high"},"model_messages":{"instructions_template":"replace policy"}}]}"#;
        let provider =
            codex_model_provider_info::built_in_model_providers(None)["deepseek"].clone();
        let model = parse_public_models(body, &provider).unwrap().remove(0);
        assert_eq!(
            levels(&model),
            vec![
                ReasoningEffort::None,
                ReasoningEffort::Low,
                ReasoningEffort::High,
                ReasoningEffort::Max
            ]
        );
        assert_eq!(model.default_reasoning_level, Some(ReasoningEffort::High));
        assert_eq!(
            model.model_messages,
            model_info_from_slug("deepseek-new-alias").model_messages
        );
        let other = parse_public_models(body, &ModelProviderInfo::default())
            .unwrap()
            .remove(0);
        assert!(levels(&other).is_empty());
    }

    #[test]
    fn known_deepseek_capabilities_do_not_leak_to_other_providers_or_models() {
        let body =
            br#"{"data":[{"id":"deepseek-flash"},{"id":"deepseek-v4-pro"},{"id":"future-model"}]}"#;
        let provider =
            codex_model_provider_info::built_in_model_providers(None)["deepseek"].clone();
        let models = parse_public_models(body, &provider).unwrap();
        assert_eq!(models[0].context_window, Some(1_000_000));
        assert_eq!(models[0].supported_reasoning_levels.len(), 4);
        assert!(models[0].input_modalities.contains(&InputModality::Image));
        assert!(!models[1].input_modalities.contains(&InputModality::Image));
        assert_eq!(models[2].context_window, None);
        assert!(models[2].supported_reasoning_levels.is_empty());
        let cloud = parse_public_models(body, &ModelProviderInfo::default()).unwrap();
        assert_eq!(cloud[0].context_window, None);
        assert!(cloud[0].supported_reasoning_levels.is_empty());
    }

    #[test]
    fn public_catalog_keeps_provider_ids_without_importing_messages_or_capabilities() {
        let models = parse_public_models(
            br#"{"data":[{"id":"example/model:cloud","model_messages":{"instructions_template":"replace policy"},"context_window":99999999}]}"#,
            &ModelProviderInfo::default(),
        )
        .unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].slug, "example/model:cloud");
        assert_eq!(models[0].context_window, None);
        assert_eq!(models[0].visibility, ModelVisibility::List);
        assert!(!models[0].supports_reasoning_summary_parameter);
        assert_eq!(
            codex_models_manager::model_info::BASE_INSTRUCTIONS,
            models[0]
                .model_messages
                .as_ref()
                .unwrap()
                .instructions_template
                .as_deref()
                .unwrap(),
        );
    }

    #[test]
    fn rejects_malformed_ambiguous_or_unusable_public_catalogs() {
        for body in [
            r#"{"models":[]}"#,
            r#"{"data":[{"id":""}]}"#,
            r#"{"data":[{"id":" model "}]}"#,
            r#"{"data":[{"id":"model"},{"id":"model"}]}"#,
            r#"{"data":[{"id":"model\n"}]}"#,
        ] {
            assert!(parse_public_models(body.as_bytes(), &ModelProviderInfo::default()).is_err());
        }
        assert!(
            parse_public_models(br#"{"data":[]}"#, &ModelProviderInfo::default())
                .unwrap()
                .is_empty()
        );
    }
}
