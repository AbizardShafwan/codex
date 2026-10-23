//! Reconnect an idle thread to another provider without discarding its durable history.

use super::session_lifecycle::ThreadAttachPresentation;
use super::*;
use crate::app_server_session::ResumeModelSettings;
use crate::chatwidget::ThreadInputStateRestoreMode;
use crate::onboarding::onboarding_screen::OnboardingScreenArgs;
use crate::onboarding::onboarding_screen::run_onboarding_app;

impl App {
    pub(super) async fn login_provider(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
    ) -> Result<()> {
        if self.chat_widget.is_external_writer_view()
            || self.reject_pending_permission_root_switch()
        {
            return Err(color_eyre::eyre::eyre!(
                "Provider switching requires a writable, idle conversation."
            ));
        }
        let (login_status, _) = crate::get_login_status(app_server).await?;
        let result = Box::pin(run_onboarding_app(
            OnboardingScreenArgs {
                show_login_screen: true,
                bedrock_setup_enabled: false,
                show_trust_screen: false,
                remote_project_trust: None,
                login_status,
                app_server_target: self.app_server_target.clone(),
                app_server_request_handle: Some(app_server.request_handle()),
                config: self.config.clone(),
            },
            Some(app_server),
            tui,
        ))
        .await?;
        if result.should_exit {
            return Ok(());
        }
        let Some(provider_id) = result.selected_provider else {
            return Ok(());
        };
        let selection = crate::provider_login::select_provider(
            app_server.request_handle(),
            provider_id,
            Some(self.chat_widget.current_model()),
        )
        .await?;
        let mut config = self.config.clone();
        config.model_provider = config
            .model_providers
            .get(&selection.provider_id)
            .cloned()
            .ok_or_else(|| {
                color_eyre::eyre::eyre!("The selected provider is not configured in this client.")
            })?;
        config.model_provider_id = selection.provider_id.clone();
        config.model = Some(selection.model.model.clone());
        config.model_reasoning_effort = (!selection.model.supported_reasoning_efforts.is_empty())
            .then_some(selection.model.default_reasoning_effort.clone());
        config.service_tier = None;
        if config.model_provider_id != self.config.model_provider_id {
            config.model_catalog = None;
        }
        let previous_override = app_server.model_provider_override.clone();
        app_server.model_provider_override = Some(selection.provider_id.clone());
        let input = self.chat_widget.capture_thread_input_state();
        let resumed = if let Some(thread_id) = self.current_displayed_thread_id() {
            // Cold resume can rebuild an idle thread only when no other client observes it.
            // A busy/shared thread keeps its existing provider; never interrupt it to switch.
            if let Err(error) = app_server.thread_unsubscribe(thread_id).await {
                app_server.model_provider_override = previous_override;
                return Err(error);
            }
            match app_server
                .resume_thread(
                    &self.local_settings,
                    config.clone(),
                    thread_id,
                    ResumeModelSettings::OverrideFromCurrentConfig,
                )
                .await
            {
                Ok(resumed)
                    if resumed.session.model_provider_id == selection.provider_id
                        && resumed.session.model == selection.model.model =>
                {
                    Some(resumed)
                }
                Ok(_) => {
                    app_server.model_provider_override = previous_override;
                    return Err(color_eyre::eyre::eyre!(
                        "The conversation is busy or shared with another client. Its provider was left unchanged."
                    ));
                }
                Err(error) => {
                    app_server.model_provider_override = previous_override;
                    // Reattach the previous route after a failed handoff so input remains usable.
                    let _ = app_server
                        .resume_thread(
                            &self.local_settings,
                            self.config.clone(),
                            thread_id,
                            ResumeModelSettings::PreserveExistingThread,
                        )
                        .await;
                    return Err(error);
                }
            }
        } else {
            None
        };

        self.config = config;
        self.harness_overrides.model_provider = Some(selection.provider_id.clone());
        self.harness_overrides.model = Some(selection.model.model.clone());
        self.model_catalog = Arc::new(
            crate::model_catalog::ModelCatalog::new(selection.models.clone())
                .with_collaboration_modes(self.model_catalog.collaboration_modes.clone()),
        );
        app_server.set_available_models(selection.models.clone());
        let persistence_error = crate::provider_login::persist_provider_selection(
            app_server.request_handle(),
            &selection,
        )
        .await
        .err();
        if let Some(resumed) = resumed {
            self.replace_chat_widget_with_app_server_thread(
                tui,
                resumed,
                ThreadAttachPresentation::SessionLineage,
                None,
            )
            .await?;
        } else {
            self.start_fresh_session(tui, app_server, None, None, None)
                .await;
        }
        self.chat_widget.restore_thread_input_state(
            input,
            ThreadInputStateRestoreMode {
                preserve_in_flight_turn: false,
            },
        );
        if let Some(error) = persistence_error {
            self.chat_widget.add_error_message(format!(
                "Connected, but failed to save provider defaults: {error}"
            ));
        }
        self.chat_widget.requires_openai_auth = self.config.model_provider.requires_openai_auth;
        self.chat_widget
            .update_account_state(None, None, false, false);
        let refresh = async {
            let account = app_server.read_account().await?;
            app_server
                .bootstrap_with_account(&self.config, account)
                .await
        }
        .await;
        match refresh {
            Ok(bootstrap) => {
                let backend_auth = self.config.model_provider.requires_openai_auth
                    && bootstrap.has_chatgpt_account;
                self.chat_widget.update_account_state(
                    bootstrap.status_account_display,
                    bootstrap.plan_type,
                    backend_auth,
                    backend_auth,
                );
            }
            Err(error) => self.chat_widget.add_error_message(format!(
                "Connected, but account controls could not be refreshed: {error}"
            )),
        }
        self.chat_widget.add_info_message(
            format!("Connected to {}. This conversation's history will be sent to the selected provider. Use /model to choose a model.", self.config.model_provider.name),
            None,
        );
        self.chat_widget.open_model_popup();
        Ok(())
    }
}
