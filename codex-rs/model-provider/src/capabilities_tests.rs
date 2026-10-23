use super::*;
use crate::create_model_provider;
use codex_model_provider_info::ModelProviderCapabilities;
use pretty_assertions::assert_eq;

#[test]
fn native_web_capabilities_are_scoped_to_known_public_providers_and_honor_opt_outs() {
    for name in ["DeepSeek", "Ollama Cloud", "Other"] {
        let info = ModelProviderInfo {
            name: name.into(),
            model_catalog_format: codex_model_provider_info::ModelCatalogFormat::Openai,
            ..Default::default()
        };
        let capabilities = create_model_provider(info.clone(), None).capabilities();
        assert_eq!(capabilities.web_search, name != "Other");
        assert_eq!(capabilities.external_web_access, name != "Other");
        assert!(!capabilities.image_generation);
        let disabled = create_model_provider(
            ModelProviderInfo {
                capabilities: Some(ModelProviderCapabilities {
                    web_search: Some(false),
                    external_web_access: Some(false),
                    ..Default::default()
                }),
                ..info
            },
            None,
        )
        .capabilities();
        assert!(!disabled.web_search);
        assert!(!disabled.external_web_access);
    }
}

#[test]
fn capability_overrides_preserve_unspecified_provider_defaults() {
    for info in [
        ModelProviderInfo::default(),
        ModelProviderInfo::create_openai_provider(/*base_url*/ None),
        ModelProviderInfo {
            name: "Azure".to_string(),
            ..Default::default()
        },
    ] {
        let defaults = create_model_provider(info.clone(), /*auth_manager*/ None).capabilities();
        let provider = create_model_provider(
            ModelProviderInfo {
                capabilities: Some(ModelProviderCapabilities {
                    external_web_access: Some(false),
                    ..Default::default()
                }),
                ..info
            },
            /*auth_manager*/ None,
        );
        assert_eq!(
            provider.capabilities(),
            ProviderCapabilities {
                external_web_access: false,
                ..defaults
            },
        );
    }
}
