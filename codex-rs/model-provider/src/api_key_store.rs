//! Provider API keys are encrypted separately from ChatGPT login and scoped to the endpoint.

use std::io;
use std::path::Path;
use std::path::PathBuf;

use codex_model_provider_info::ModelProviderInfo;
use codex_secrets::SecretName;
use codex_secrets::SecretScope;
use codex_secrets::SecretsBackendKind;
use codex_secrets::SecretsManager;
use sha2::Digest;
use sha2::Sha256;

pub struct ProviderApiKeyStore {
    secrets: SecretsManager,
}

impl ProviderApiKeyStore {
    pub fn new(codex_home: &Path, provider: &ModelProviderInfo) -> io::Result<Self> {
        Ok(Self {
            secrets: SecretsManager::new(
                store_directory(codex_home, provider)?,
                SecretsBackendKind::Local,
            ),
        })
    }

    pub fn load(&self) -> io::Result<Option<String>> {
        self.secrets
            .get(&SecretScope::Global, &key_name()?)
            .map_err(|_| io::Error::other("failed to load the provider API key"))
    }

    pub fn save(&self, api_key: &str) -> io::Result<()> {
        let api_key = api_key.trim();
        if api_key.is_empty() || http::HeaderValue::from_str(api_key).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the API key must be non-empty and contain no invalid header characters",
            ));
        }
        self.secrets
            .set(&SecretScope::Global, &key_name()?, api_key)
            .map_err(|_| io::Error::other("failed to save the provider API key"))
    }

    pub fn delete(&self) -> io::Result<bool> {
        self.secrets
            .delete(&SecretScope::Global, &key_name()?)
            .map_err(|_| io::Error::other("failed to remove the provider API key"))
    }
}

fn store_directory(codex_home: &Path, provider: &ModelProviderInfo) -> io::Result<PathBuf> {
    let endpoint = provider
        .base_url
        .as_deref()
        .ok_or_else(|| io::Error::other("provider API-key storage requires a base URL"))?;
    let endpoint = url::Url::parse(endpoint)
        .map_err(|_| io::Error::other("provider API-key storage requires a valid base URL"))?;
    if !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
    {
        return Err(io::Error::other(
            "provider API-key storage requires an HTTP endpoint without URL credentials",
        ));
    }
    let mut digest = Sha256::new();
    for field in [provider.name.as_bytes(), endpoint.as_str().as_bytes()] {
        digest.update((field.len() as u64).to_le_bytes());
        digest.update(field);
    }
    Ok(codex_home
        .join("provider-auth")
        .join(format!("{:x}", digest.finalize())))
}

fn key_name() -> io::Result<SecretName> {
    SecretName::new("API_KEY").map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_keyring_store::tests::MockKeyringStore;
    use std::sync::Arc;

    #[test]
    fn saved_keys_are_isolated_by_provider_and_endpoint() {
        let home = tempfile::tempdir().unwrap();
        let keyring = Arc::new(MockKeyringStore::default());
        let provider = ModelProviderInfo {
            name: "A".into(),
            base_url: Some("https://a.example/v1".into()),
            ..Default::default()
        };
        let store = |provider: &ModelProviderInfo| ProviderApiKeyStore {
            secrets: SecretsManager::new_with_keyring_store(
                store_directory(home.path(), provider).unwrap(),
                SecretsBackendKind::Local,
                keyring.clone(),
            ),
        };
        store(&provider).save("only-A").unwrap();
        let other_endpoint = ModelProviderInfo {
            base_url: Some("https://b.example/v1".into()),
            ..provider.clone()
        };
        let other_provider = ModelProviderInfo {
            name: "B".into(),
            ..provider.clone()
        };
        assert_eq!(store(&provider).load().unwrap().as_deref(), Some("only-A"));
        assert!(store(&other_endpoint).load().unwrap().is_none());
        assert!(store(&other_provider).load().unwrap().is_none());
        assert!(!home.path().join("auth.json").exists());
    }

    #[test]
    fn api_key_store_round_trips_and_rejects_header_injection() {
        let home = tempfile::tempdir().unwrap();
        let store = ProviderApiKeyStore {
            secrets: SecretsManager::new_with_keyring_store(
                home.path().to_path_buf(),
                SecretsBackendKind::Local,
                Arc::new(MockKeyringStore::default()),
            ),
        };
        assert!(store.load().unwrap().is_none());
        for key in ["", "  ", "token\r\nInjected: value"] {
            assert!(store.save(key).is_err());
        }
        store.save(" provider-token ").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("provider-token"));
        assert!(store.delete().unwrap());
        assert!(store.load().unwrap().is_none());
    }
}
