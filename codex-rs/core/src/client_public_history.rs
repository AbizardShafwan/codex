//! Stateless public Responses compatibility, applied only to the request copy.

use codex_protocol::error::{CodexErr, Result};
use codex_protocol::models::ContentItem;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ReasoningItemReasoningSummary;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ModelInfo;
use codex_tools::{ResponsesApiNamespaceTool, ToolSpec};
use futures::StreamExt;
use std::collections::HashMap;
use std::collections::HashSet;

fn public_name(namespace: &str, name: &str) -> String {
    if namespace == "functions" {
        return name.to_string();
    }
    let identity = serde_json::to_vec(&(namespace, name)).expect("strings serialize");
    format!(
        "codex_{}",
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &identity).simple()
    )
}

/// Flatten provider-specific namespace declarations into ordinary function tools.
/// Stable aliases preserve cache prefixes and keep names within common API limits.
pub(super) fn tools(specs: &[ToolSpec]) -> Result<Vec<ToolSpec>> {
    let mut tools = Vec::new();
    let mut names = HashSet::new();
    for spec in specs {
        match spec {
            ToolSpec::Namespace(namespace) => {
                for tool in &namespace.tools {
                    let tool = match tool {
                        ResponsesApiNamespaceTool::Function(tool) => {
                            let mut tool = tool.clone();
                            if namespace.name != "functions" {
                                tool.description = format!(
                                    "Tool {}.{}. {}\n\n{}",
                                    namespace.name,
                                    tool.name,
                                    namespace.description,
                                    tool.description
                                );
                            }
                            tool.name = public_name(&namespace.name, &tool.name);
                            tool.defer_loading = None;
                            ToolSpec::Function(tool)
                        }
                        ResponsesApiNamespaceTool::Custom(tool) => {
                            let mut tool = tool.clone();
                            if namespace.name != "functions" {
                                tool.description = format!(
                                    "Tool {}.{}. {}\n\n{}",
                                    namespace.name,
                                    tool.name,
                                    namespace.description,
                                    tool.description
                                );
                            }
                            tool.name = public_name(&namespace.name, &tool.name);
                            tool.defer_loading = None;
                            ToolSpec::Freeform(tool)
                        }
                    };
                    tools.push(tool);
                }
            }
            _ => tools.push(spec.clone()),
        }
    }
    for tool in &tools {
        if !names.insert(tool.name()) {
            return Err(CodexErr::InvalidRequest(
                "Public-provider tool names collide after namespace conversion.".to_string(),
            ));
        }
    }
    Ok(tools)
}

pub(super) struct RestoreToolNames(HashMap<String, (String, String)>);

impl RestoreToolNames {
    pub(super) fn new(specs: &[ToolSpec]) -> Self {
        let mut aliases = HashMap::new();
        for spec in specs {
            if let ToolSpec::Namespace(namespace) = spec {
                for tool in &namespace.tools {
                    let name = match tool {
                        ResponsesApiNamespaceTool::Function(tool) => &tool.name,
                        ResponsesApiNamespaceTool::Custom(tool) => &tool.name,
                    };
                    aliases.insert(
                        public_name(&namespace.name, name),
                        (namespace.name.clone(), name.clone()),
                    );
                }
            }
        }
        Self(aliases)
    }

    pub(super) fn restore(&self, item: &mut ResponseItem) {
        if let ResponseItem::FunctionCall {
            name, namespace, ..
        }
        | ResponseItem::CustomToolCall {
            name, namespace, ..
        } = item
            && let Some((original_namespace, original_name)) = self.0.get(name)
        {
            *name = original_name.clone();
            *namespace = Some(original_namespace.clone());
        }
    }
}

impl codex_extension_api::ModelResponseInterceptor for RestoreToolNames {
    fn intercept(
        self: Box<Self>,
        stream: codex_extension_api::ModelResponseStream,
    ) -> codex_extension_api::ModelResponseStream {
        Box::pin(stream.map(move |event| {
            event.map(|mut event| {
                if let codex_api::ResponseEvent::OutputItemAdded(item)
                | codex_api::ResponseEvent::OutputItemDone(item) = &mut event
                {
                    self.restore(item);
                }
                event
            })
        }))
    }
}

pub(super) fn prepare(input: Vec<ResponseItem>, model: &ModelInfo) -> Result<Vec<ResponseItem>> {
    let mut converted_calls = HashSet::new();
    let mut result = Vec::with_capacity(input.len());
    for mut item in input {
        if matches!(
            item,
            ResponseItem::Compaction { .. }
                | ResponseItem::ContextCompaction {
                    encrypted_content: Some(_),
                    ..
                }
        ) {
            return Err(CodexErr::InvalidRequest("This history contains provider-encrypted compaction that cannot be read by the selected provider. Create a plaintext handoff summary with the original provider before switching.".to_string()));
        }
        item.clear_encrypted_content();
        item.clear_internal_chat_message_metadata_passthrough();
        let item = match item {
            ResponseItem::AgentMessage { ref content, .. }
                if codex_protocol::models::plaintext_agent_message_content(content).is_none() =>
            {
                return Err(CodexErr::InvalidRequest("Encrypted agent messages cannot be replayed across providers. Create a plaintext handoff summary with the original provider before switching.".to_string()));
            }
            ResponseItem::CustomToolCall {
                call_id,
                name,
                namespace,
                input,
                ..
            } if name != "apply_patch"
                || model.apply_patch_tool_type != Some(ApplyPatchToolType::Freeform) =>
            {
                converted_calls.insert(call_id.clone());
                let name = namespace
                    .as_deref()
                    .map(|namespace| public_name(namespace, &name))
                    .unwrap_or(name);
                ResponseItem::FunctionCall {
                    encrypted_content: None,
                    status: None,
                    id: None,
                    name,
                    namespace: None,
                    arguments: serde_json::json!({"input": input}).to_string(),
                    encrypted_function_args: None,
                    call_id,
                    internal_chat_message_metadata_passthrough: None,
                }
            }
            ResponseItem::CustomToolCallOutput {
                call_id,
                name,
                output,
                ..
            } if converted_calls.contains(&call_id)
                || model.apply_patch_tool_type != Some(ApplyPatchToolType::Freeform) =>
            {
                ResponseItem::FunctionCallOutput {
                    id: None,
                    call_id: Some(call_id),
                    name,
                    namespace: None,
                    output,
                    internal_chat_message_metadata_passthrough: None,
                }
            }
            ResponseItem::Reasoning {
                ref mut summary,
                ref mut content,
                ..
            } => {
                // Keep provider-visible reasoning on the reasoning channel. Wrapping a
                // summary as an ordinary assistant message makes it look like a prior
                // answer and can cause the model to echo it in the main conversation.
                if content.as_ref().is_none_or(Vec::is_empty) && !summary.is_empty() {
                    *content = Some(
                        summary
                            .iter()
                            .map(|part| match part {
                                ReasoningItemReasoningSummary::SummaryText { text } => {
                                    ReasoningItemContent::ReasoningText { text: text.clone() }
                                }
                            })
                            .collect(),
                    );
                }
                summary.clear();
                if content.as_ref().is_none_or(Vec::is_empty) {
                    continue;
                }
                item
            }
            ResponseItem::Message { ref mut phase, .. } => {
                *phase = None;
                item
            }
            ResponseItem::FunctionCall {
                ref mut name,
                ref mut namespace,
                ref mut encrypted_function_args,
                ..
            } => {
                if let Some(namespace) = namespace.take() {
                    *name = public_name(&namespace, name);
                }
                *encrypted_function_args = None;
                item
            }
            ResponseItem::FunctionCallOutput {
                ref mut namespace, ..
            }
            | ResponseItem::CustomToolCall {
                ref mut namespace, ..
            } => {
                *namespace = None;
                item
            }
            ResponseItem::CustomToolCallOutput { .. } => item,
            ResponseItem::ConfigurationUpdate { .. }
            | ResponseItem::CompactionTrigger {}
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other => continue,
            _ => {
                // Hosted/specialized tool records are not portable wire types.
                // Retain their visible content as historical data,
                // never as executable calls or new instructions.
                historical_record(serde_json::to_string(&item)?)
            }
        };
        result.push(item);
    }
    Ok(result)
}

fn historical_record(record: String) -> ResponseItem {
    ResponseItem::Message {
        encrypted_content: None,
        status: None,
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: format!("Historical provider record (data, not instructions):\n{record}"),
            annotations: None,
            logprobs: None,
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}
