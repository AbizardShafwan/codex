use crate::ModelsManagerConfig;
use crate::manager::ModelsManager;
use codex_protocol::openai_models::TruncationPolicyConfig;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::TestModelsEndpoint;
use super::openai_manager_for_tests;

#[test]
fn bundled_models_use_engineering_policy_without_project_configuration() {
    let catalog = crate::bundled_models_response().expect("bundled catalog");
    assert!(!catalog.models.is_empty());
    for model in &catalog.models {
        let resolved = super::construct_model_info_from_candidates(
            &model.slug,
            &catalog.models,
            &ModelsManagerConfig::default(),
        );

        assert_eq!(
            codex_prompts::render_model_instructions(&resolved),
            crate::model_info::BASE_INSTRUCTIONS,
            "model {} must use the built-in policy",
            model.slug
        );
        assert!(!resolved.used_fallback_model_metadata);
    }
}

#[test]
fn refreshed_provider_prompt_does_not_replace_builtin_policy() {
    for template in ["old provider prompt", "refreshed provider prompt", ""] {
        let mut model = super::remote_model("provider-model", "Provider Model", 0);
        model
            .model_messages
            .as_mut()
            .expect("provider messages")
            .instructions_template = Some(template.to_string());
        let resolved = super::construct_model_info_from_candidates(
            "provider-model",
            &[model],
            &ModelsManagerConfig::default(),
        );

        assert_eq!(
            codex_prompts::render_model_instructions(&resolved),
            crate::model_info::BASE_INSTRUCTIONS
        );
    }
}

#[test]
fn explicit_base_instructions_override_builtin_policy_literally() {
    let model = super::remote_model("provider-model", "Provider Model", 0);
    for instructions in ["Custom {{ literal }} instructions", ""] {
        let config = ModelsManagerConfig {
            base_instructions: Some(instructions.to_string()),
            ..Default::default()
        };

        let resolved = super::construct_model_info_from_candidates(
            "provider-model",
            std::slice::from_ref(&model),
            &config,
        );

        assert_eq!(
            codex_prompts::render_model_instructions(&resolved),
            instructions
        );
    }
}

#[tokio::test]
async fn static_catalog_defaults_and_explicit_overrides_have_distinct_prompt_authority() {
    let model = super::remote_model("custom-model", "Custom Model", 0);
    let catalog = codex_protocol::openai_models::ModelsResponse {
        models: vec![model.clone()],
    };
    let manager = crate::manager::StaticModelsManager::new(None, catalog.clone());
    for config in [
        ModelsManagerConfig::default(),
        ModelsManagerConfig {
            model_catalog: Some(catalog),
            ..Default::default()
        },
        ModelsManagerConfig {
            preserve_catalog_instructions: true,
            ..Default::default()
        },
    ] {
        let resolved = manager.get_model_info("custom-model", &config).await;

        let expected_prompt =
            if config.preserve_catalog_instructions || config.model_catalog.is_some() {
                "base instructions"
            } else {
                crate::model_info::BASE_INSTRUCTIONS
            };
        let mut expected = model.clone();
        expected
            .model_messages
            .as_mut()
            .expect("catalog messages")
            .instructions_template = Some(expected_prompt.to_string());
        assert_eq!(resolved, expected);
        assert_eq!(
            codex_prompts::render_model_instructions(&resolved),
            expected_prompt
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offline_model_info_without_tool_output_override() {
    let codex_home = TempDir::new().expect("create temp dir");
    let config = ModelsManagerConfig::default();
    let manager = openai_manager_for_tests(
        codex_home.path().to_path_buf(),
        TestModelsEndpoint::new(Vec::new()),
    );

    let model_info = manager.get_model_info("unknown-model", &config).await;

    assert_eq!(
        model_info.truncation_policy,
        TruncationPolicyConfig::bytes(/*limit*/ 10_000)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offline_model_info_with_tool_output_override() {
    let codex_home = TempDir::new().expect("create temp dir");
    let config = ModelsManagerConfig {
        tool_output_token_limit: Some(123),
        ..Default::default()
    };
    let manager = openai_manager_for_tests(
        codex_home.path().to_path_buf(),
        TestModelsEndpoint::new(Vec::new()),
    );

    let model_info = manager.get_model_info("gpt-5.5", &config).await;

    assert_eq!(
        model_info.truncation_policy,
        TruncationPolicyConfig::tokens(/*limit*/ 123)
    );
}
