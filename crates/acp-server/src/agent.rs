use crate::types::{AgentManifest, AgentMetadata, AgentStatus, Capability};

/// Build the Ante ACP agent manifest.
pub fn build_manifest(stats: Option<AgentStatus>) -> AgentManifest {
    AgentManifest {
        name: "ante".to_string(),
        description: "Self-contained Rust coding agent with multi-provider LLM support, tool execution, file editing, shell commands, code search, git operations, MCP integration, and multi-agent orchestration.".to_string(),
        input_content_types: vec!["text/plain".to_string()],
        output_content_types: vec!["text/plain".to_string()],
        metadata: Some(AgentMetadata {
            programming_language: Some("Rust".to_string()),
            framework: Some("Ante".to_string()),
            license: Some("Apache-2.0".to_string()),
            natural_languages: Some(vec!["en".to_string()]),
            capabilities: Some(vec![
                Capability { name: "Code Generation & Editing".into(), description: "Read, write, and edit files with full AST awareness".into() },
                Capability { name: "Shell Execution".into(), description: "Run shell commands with configurable permission modes".into() },
                Capability { name: "Git Operations".into(), description: "Embedded git for version control workflows".into() },
                Capability { name: "Code Search".into(), description: "Embedded grep and file search across codebases".into() },
                Capability { name: "Multi-provider LLM".into(), description: "12+ providers: Anthropic, OpenAI, Gemini, Grok, OpenRouter, local GGUF and more".into() },
                Capability { name: "Offline Inference".into(), description: "Built-in llama.cpp engine for local model execution".into() },
                Capability { name: "MCP Integration".into(), description: "Model Context Protocol tool integration".into() },
                Capability { name: "Multi-agent Orchestration".into(), description: "Spawn and coordinate sub-agents for complex tasks".into() },
            ]),
            tags: Some(vec!["Code".to_string(), "Orchestrator".to_string()]),
            recommended_models: Some(vec![
                "claude-sonnet-4-6".to_string(),
                "gpt-5".to_string(),
                "gemini-3-pro".to_string(),
            ]),
            annotations: None,
        }),
        status: stats,
    }
}
