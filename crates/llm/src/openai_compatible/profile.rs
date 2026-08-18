use crate::effort;
use ante_protocol_shape::Effort;
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenAiCompatProfile {
    thinking_dialect: ThinkingDialect,
    system_role: SystemRolePolicy,
    search: SearchPolicy,
    send_images: bool,
    requires_assistant_reasoning: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingDialect {
    ReasoningEffort,
    OpenRouterHighXHigh,
    ThinkingObject,
    KimiThinkingObject,
    KimiK3ReasoningEffort,
    MuseSparkReasoningEffort,
    Qwen38ReasoningEffort,
    QwenEnableThinking,
    DeepSeek { max_reasoning_effort: ReasoningEffort },
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemRolePolicy {
    Separate,
    Merged,
    PrependToFirstUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchPolicy {
    Unsupported,
    OpenRouterWebSearchTool,
    QwenEnableSearch,
}

#[derive(Debug)]
pub struct ThinkingParams {
    pub reasoning_effort: Option<ReasoningEffort>,
    pub thinking: Option<ThinkingConfig>,
    pub enable_thinking: Option<bool>,
    pub thinking_budget: Option<u32>,
}

#[derive(Debug)]
pub struct SearchParams {
    pub enable_search: Option<bool>,
    pub search_options: Option<SearchOptions>,
    pub enable_thinking: Option<bool>,
    pub extra_body: Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct SearchOptions {
    forced_search: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    None,
    Minimal,
    #[serde(rename = "xhigh")]
    XHigh,
    Low,
    Medium,
    High,
    Max,
}

/// Provider-native thinking configuration.
#[derive(Debug, Serialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    thinking_type: String,
}

impl ThinkingConfig {
    pub fn thinking_type(&self) -> &str {
        &self.thinking_type
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenAiCompatFamily {
    DeepSeek,
    Glm,
    Kimi,
    KimiK3,
    MuseSpark,
    MiniMax,
    Qwen,
    MistralNoSystem,
    Generic,
}

impl OpenAiCompatProfile {
    pub fn from_model(provider_id: &str, model_id: &str, send_images: bool) -> Self {
        let family = OpenAiCompatFamily::from_provider_model(provider_id, model_id);
        Self::from_family(family, send_images, provider_id, model_id)
    }

    fn from_family(
        family: OpenAiCompatFamily,
        send_images: bool,
        provider_id: &str,
        model_id: &str,
    ) -> Self {
        Self {
            thinking_dialect: family.thinking_dialect(provider_id, model_id),
            system_role: family.system_role(),
            search: family.search_policy(provider_id),
            send_images,
            requires_assistant_reasoning: family.requires_assistant_reasoning(),
        }
    }

    pub fn thinking_dialect(self) -> ThinkingDialect {
        self.thinking_dialect
    }

    pub fn thinking_params(self, requested: Option<Effort>) -> ThinkingParams {
        match self.thinking_dialect {
            ThinkingDialect::ThinkingObject => ThinkingParams {
                reasoning_effort: None,
                thinking: Some(thinking_object(thinking_on(requested))),
                enable_thinking: None,
                thinking_budget: None,
            },
            ThinkingDialect::KimiThinkingObject => ThinkingParams {
                reasoning_effort: None,
                // Kimi K2.x only takes an explicit "disabled" object; on is the default.
                thinking: (!thinking_on(requested)).then(|| thinking_object(false)),
                enable_thinking: None,
                thinking_budget: None,
            },
            ThinkingDialect::KimiK3ReasoningEffort => ThinkingParams {
                reasoning_effort: requested.map(|r| effort::resolve(KIMI_K3_RUNGS, r)),
                thinking: None,
                enable_thinking: None,
                thinking_budget: None,
            },
            ThinkingDialect::MuseSparkReasoningEffort => ThinkingParams {
                reasoning_effort: requested.map(|r| effort::resolve(MUSE_SPARK_RUNGS, r)),
                thinking: None,
                enable_thinking: None,
                thinking_budget: None,
            },
            ThinkingDialect::Qwen38ReasoningEffort => {
                let setting = requested.map(|r| effort::resolve(QWEN38_RUNGS, r));
                let (reasoning_effort, enable_thinking) = match setting {
                    Some(Qwen38Setting::ThinkingOff) => (None, Some(false)),
                    Some(Qwen38Setting::ReasoningEffort(effort)) => (Some(effort), None),
                    None => (None, None),
                };
                ThinkingParams {
                    reasoning_effort,
                    thinking: None,
                    enable_thinking,
                    thinking_budget: None,
                }
            }
            ThinkingDialect::QwenEnableThinking => ThinkingParams {
                reasoning_effort: None,
                thinking: None,
                enable_thinking: Some(thinking_on(requested)),
                thinking_budget: None,
            },
            ThinkingDialect::DeepSeek { max_reasoning_effort } => ThinkingParams {
                reasoning_effort: requested
                    .and_then(|r| effort::resolve(&deepseek_rungs(max_reasoning_effort), r)),
                thinking: Some(thinking_object(thinking_on(requested))),
                enable_thinking: None,
                thinking_budget: None,
            },
            ThinkingDialect::ReasoningEffort => ThinkingParams {
                reasoning_effort: requested
                    .and_then(|r| effort::resolve(REASONING_EFFORT_RUNGS, r)),
                thinking: None,
                enable_thinking: None,
                thinking_budget: None,
            },
            ThinkingDialect::OpenRouterHighXHigh => ThinkingParams {
                reasoning_effort: requested.map(|r| effort::resolve(OPENROUTER_RUNGS, r)),
                thinking: None,
                enable_thinking: None,
                thinking_budget: None,
            },
            ThinkingDialect::None => ThinkingParams {
                reasoning_effort: None,
                thinking: None,
                enable_thinking: None,
                thinking_budget: None,
            },
        }
    }

    pub fn search_params(self) -> SearchParams {
        match self.search {
            SearchPolicy::OpenRouterWebSearchTool => SearchParams {
                enable_search: None,
                search_options: None,
                enable_thinking: None,
                extra_body: openrouter_web_search_extra_body(),
            },
            SearchPolicy::QwenEnableSearch => SearchParams {
                enable_search: Some(true),
                search_options: Some(SearchOptions { forced_search: true }),
                enable_thinking: Some(false),
                extra_body: Map::new(),
            },
            SearchPolicy::Unsupported => SearchParams {
                enable_search: None,
                search_options: None,
                enable_thinking: None,
                extra_body: Map::new(),
            },
        }
    }

    pub fn supports_search(self) -> bool {
        !matches!(self.search, SearchPolicy::Unsupported)
    }

    pub fn supports_system_role(self) -> bool {
        !matches!(self.system_role, SystemRolePolicy::PrependToFirstUser)
    }

    pub fn merges_system_messages(self) -> bool {
        matches!(self.system_role, SystemRolePolicy::Merged)
    }

    pub fn sends_images(self) -> bool {
        self.send_images
    }

    pub fn requires_assistant_reasoning(self) -> bool {
        self.requires_assistant_reasoning
    }
}

impl OpenAiCompatFamily {
    fn thinking_dialect(self, provider_id: &str, model_id: &str) -> ThinkingDialect {
        match self {
            Self::DeepSeek => ThinkingDialect::DeepSeek {
                max_reasoning_effort: deepseek_max_reasoning_effort(provider_id, model_id),
            },
            Self::Glm if provider_id.eq_ignore_ascii_case("openrouter") => {
                ThinkingDialect::OpenRouterHighXHigh
            }
            Self::Glm => ThinkingDialect::ThinkingObject,
            Self::Kimi => ThinkingDialect::KimiThinkingObject,
            Self::KimiK3 => ThinkingDialect::KimiK3ReasoningEffort,
            Self::MuseSpark => ThinkingDialect::MuseSparkReasoningEffort,
            Self::MiniMax => ThinkingDialect::None,
            Self::Qwen if is_qwen38(model_id) => ThinkingDialect::Qwen38ReasoningEffort,
            Self::Qwen => ThinkingDialect::QwenEnableThinking,
            Self::MistralNoSystem | Self::Generic => ThinkingDialect::ReasoningEffort,
        }
    }

    fn system_role(self) -> SystemRolePolicy {
        match self {
            // Unknown OpenAI-compatible models may use strict local chat
            // templates that only accept one leading system message. Merge by
            // default; keep separate messages only for families known to
            // support them.
            Self::MiniMax | Self::Qwen | Self::Generic => SystemRolePolicy::Merged,
            Self::MistralNoSystem => SystemRolePolicy::PrependToFirstUser,
            Self::DeepSeek | Self::Glm | Self::Kimi | Self::KimiK3 | Self::MuseSpark => {
                SystemRolePolicy::Separate
            }
        }
    }

    fn search_policy(self, provider_id: &str) -> SearchPolicy {
        if provider_id.eq_ignore_ascii_case("openrouter") {
            return SearchPolicy::OpenRouterWebSearchTool;
        }

        match self {
            Self::Qwen => SearchPolicy::QwenEnableSearch,
            Self::DeepSeek
            | Self::Glm
            | Self::Kimi
            | Self::KimiK3
            | Self::MuseSpark
            | Self::MiniMax
            | Self::MistralNoSystem
            | Self::Generic => SearchPolicy::Unsupported,
        }
    }

    fn requires_assistant_reasoning(self) -> bool {
        matches!(self, Self::DeepSeek)
    }

    fn from_provider_model(provider_id: &str, model_id: &str) -> Self {
        let provider = provider_id.to_ascii_lowercase();
        if provider == "deepseek" {
            return Self::DeepSeek;
        }
        if provider == "zai" {
            return Self::Glm;
        }

        let model = model_id.to_ascii_lowercase();
        let (owner, name) = model.split_once('/').unwrap_or(("", model.as_str()));

        if owner == "deepseek" || starts_with_family_token(name, "deepseek") {
            return Self::DeepSeek;
        }
        if owner == "qwen" || starts_with_family_token(name, "qwen") {
            return Self::Qwen;
        }
        if starts_with_family_token(name, "glm") {
            return Self::Glm;
        }
        if starts_with_family_token(name, "kimi-k3") {
            return Self::KimiK3;
        }
        if owner == "meta" && starts_with_family_token(name, "muse-spark") {
            return Self::MuseSpark;
        }
        if (owner == "moonshotai" && starts_with_family_token(name, "kimi"))
            || starts_with_family_token(name, "kimi")
        {
            return Self::Kimi;
        }
        if owner == "minimax" || starts_with_family_token(name, "minimax") {
            return Self::MiniMax;
        }
        if owner == "mistralai"
            || starts_with_family_token(name, "mistral")
            || starts_with_family_token(name, "devstral")
            || starts_with_family_token(name, "ministral")
        {
            return Self::MistralNoSystem;
        }

        Self::Generic
    }
}

fn openrouter_web_search_extra_body() -> Map<String, Value> {
    let mut body = Map::new();
    body.insert("tools".to_string(), serde_json::json!([{ "type": "openrouter:web_search" }]));
    body
}

/// On/off ladder shared by the binary dialects (GLM, Kimi K2.x, and Qwen
/// before 3.8): `min` is off, anything above is on. An unset effort defaults
/// to on, matching each dialect's historical behavior.
const ON_OFF_RUNGS: &[(Effort, bool)] = &[(Effort::Min, false), (Effort::Low, true)];

/// Kimi K3 always reasons and currently accepts only the top-level `max`
/// reasoning effort. A one-rung ladder also keeps lower internal requests on
/// that mandatory floor until Moonshot exposes more levels.
const KIMI_K3_RUNGS: &[(Effort, ReasoningEffort)] = &[(Effort::Max, ReasoningEffort::Max)];

/// Muse Spark 1.1 always reasons. OpenRouter exposes five named levels, with
/// `minimal` as the mandatory floor and `xhigh` as the ceiling.
const MUSE_SPARK_RUNGS: &[(Effort, ReasoningEffort)] = &[
    (Effort::Min, ReasoningEffort::Minimal),
    (Effort::Low, ReasoningEffort::Low),
    (Effort::Medium, ReasoningEffort::Medium),
    (Effort::High, ReasoningEffort::High),
    (Effort::XHigh, ReasoningEffort::XHigh),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Qwen38Setting {
    ThinkingOff,
    ReasoningEffort(ReasoningEffort),
}

/// Qwen 3.8 accepts graded `reasoning_effort` values while retaining
/// `enable_thinking: false` as its explicit off switch.
const QWEN38_RUNGS: &[(Effort, Qwen38Setting)] = &[
    (Effort::Min, Qwen38Setting::ThinkingOff),
    (Effort::Low, Qwen38Setting::ReasoningEffort(ReasoningEffort::Low)),
    (Effort::Medium, Qwen38Setting::ReasoningEffort(ReasoningEffort::Medium)),
    (Effort::XHigh, Qwen38Setting::ReasoningEffort(ReasoningEffort::XHigh)),
];

fn thinking_on(requested: Option<Effort>) -> bool {
    requested.is_none_or(|r| effort::resolve(ON_OFF_RUNGS, r))
}

fn thinking_object(on: bool) -> ThinkingConfig {
    let thinking_type = if on { "enabled" } else { "disabled" };
    ThinkingConfig { thinking_type: thinking_type.to_string() }
}

/// Generic `reasoning_effort` ladder. Capped at `high`: arbitrary
/// OpenAI-compatible servers commonly reject values above it, so `xhigh` and
/// `max` round down.
const REASONING_EFFORT_RUNGS: &[(Effort, Option<ReasoningEffort>)] = &[
    (Effort::Min, None),
    (Effort::Low, Some(ReasoningEffort::Low)),
    (Effort::Medium, Some(ReasoningEffort::Medium)),
    (Effort::High, Some(ReasoningEffort::High)),
];

/// OpenRouter normalizes effort across models and accepts the full range,
/// including an explicit `"none"`; `max` rounds down to `xhigh`.
const OPENROUTER_RUNGS: &[(Effort, ReasoningEffort)] = &[
    (Effort::Min, ReasoningEffort::None),
    (Effort::Low, ReasoningEffort::Low),
    (Effort::Medium, ReasoningEffort::Medium),
    (Effort::High, ReasoningEffort::High),
    (Effort::XHigh, ReasoningEffort::XHigh),
];

fn deepseek_max_reasoning_effort(provider_id: &str, model_id: &str) -> ReasoningEffort {
    // OpenRouter's April preview accepted `xhigh`; the 0813 GA model advertises
    // DeepSeek's native `max` spelling.
    if provider_id.eq_ignore_ascii_case("openrouter")
        && !model_id.eq_ignore_ascii_case("deepseek/deepseek-v4-pro-0813")
    {
        ReasoningEffort::XHigh
    } else {
        ReasoningEffort::Max
    }
}

/// DeepSeek ladder. Only `high` and the provider's top value are known-good
/// on this API, so the middle of the scale rides on `high` (named `medium`
/// here, its lowest internal level) and `max` takes the top; `min`/`low`
/// omit the parameter (server default).
fn deepseek_rungs(max_reasoning_effort: ReasoningEffort) -> [(Effort, Option<ReasoningEffort>); 3] {
    [
        (Effort::Min, None),
        (Effort::Medium, Some(ReasoningEffort::High)),
        (Effort::Max, Some(max_reasoning_effort)),
    ]
}

/// The distinct effort levels a compat provider+model exposes, for the UI.
pub fn effort_levels(provider_id: &str, model_id: &str) -> Vec<Effort> {
    let family = OpenAiCompatFamily::from_provider_model(provider_id, model_id);
    match family.thinking_dialect(provider_id, model_id) {
        ThinkingDialect::ThinkingObject
        | ThinkingDialect::KimiThinkingObject
        | ThinkingDialect::QwenEnableThinking => effort::levels(ON_OFF_RUNGS),
        ThinkingDialect::Qwen38ReasoningEffort => effort::levels(QWEN38_RUNGS),
        ThinkingDialect::KimiK3ReasoningEffort => effort::levels(KIMI_K3_RUNGS),
        ThinkingDialect::MuseSparkReasoningEffort => effort::levels(MUSE_SPARK_RUNGS),
        ThinkingDialect::DeepSeek { max_reasoning_effort } => {
            effort::levels(&deepseek_rungs(max_reasoning_effort))
        }
        ThinkingDialect::ReasoningEffort => effort::levels(REASONING_EFFORT_RUNGS),
        ThinkingDialect::OpenRouterHighXHigh => effort::levels(OPENROUTER_RUNGS),
        ThinkingDialect::None => Vec::new(),
    }
}

fn is_qwen38(model_id: &str) -> bool {
    let model = model_id.to_ascii_lowercase();
    let name = model.split_once('/').map_or(model.as_str(), |(_, name)| name);
    let Some(rest) = name.strip_prefix("qwen3.8") else {
        return false;
    };
    rest.is_empty() || rest.chars().next().is_some_and(|c| matches!(c, '-' | '_' | ':' | '.'))
}

fn starts_with_family_token(name: &str, family: &str) -> bool {
    let Some(rest) = name.strip_prefix(family) else {
        return false;
    };
    rest.is_empty()
        || rest
            .chars()
            .next()
            .is_some_and(|c| matches!(c, '-' | '_' | ':' | '.') || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(provider_id: &str, model_id: &str) -> OpenAiCompatProfile {
        OpenAiCompatProfile::from_model(provider_id, model_id, false)
    }

    #[test]
    fn explicit_family_profiles_drive_wire_policy() {
        let deepseek = profile("openrouter", "deepseek/deepseek-v4-pro");
        assert_eq!(
            deepseek.thinking_dialect,
            ThinkingDialect::DeepSeek { max_reasoning_effort: ReasoningEffort::XHigh }
        );
        assert_eq!(deepseek.system_role, SystemRolePolicy::Separate);
        assert!(deepseek.requires_assistant_reasoning);

        let deepseek_ga = profile("openrouter", "deepseek/deepseek-v4-pro-0813");
        assert_eq!(
            deepseek_ga.thinking_dialect,
            ThinkingDialect::DeepSeek { max_reasoning_effort: ReasoningEffort::Max }
        );

        let qwen = profile("openai-compatible", "qwen3.7-max");
        assert_eq!(qwen.thinking_dialect, ThinkingDialect::QwenEnableThinking);
        assert_eq!(qwen.system_role, SystemRolePolicy::Merged);
        assert_eq!(qwen.search, SearchPolicy::QwenEnableSearch);

        let qwen38 = profile("openai-compatible", "Qwen3.8-27B-MLX");
        assert_eq!(qwen38.thinking_dialect, ThinkingDialect::Qwen38ReasoningEffort);
        assert_eq!(qwen38.system_role, SystemRolePolicy::Merged);
        assert_eq!(qwen38.search, SearchPolicy::QwenEnableSearch);

        let minimax = profile("openrouter", "minimax/minimax-m3");
        assert_eq!(minimax.thinking_dialect, ThinkingDialect::None);
        assert_eq!(minimax.system_role, SystemRolePolicy::Merged);

        let openrouter_glm = profile("openrouter", "z-ai/glm-5.2");
        assert_eq!(openrouter_glm.thinking_dialect, ThinkingDialect::OpenRouterHighXHigh);

        let kimi = profile("openai-compatible", "moonshotai/kimi-k2.6");
        assert_eq!(kimi.thinking_dialect, ThinkingDialect::KimiThinkingObject);

        let kimi_k3 = profile("openrouter", "moonshotai/kimi-k3");
        assert_eq!(kimi_k3.thinking_dialect, ThinkingDialect::KimiK3ReasoningEffort);

        let muse_spark = profile("openrouter", "meta/muse-spark-1.2");
        assert_eq!(muse_spark.thinking_dialect, ThinkingDialect::MuseSparkReasoningEffort);

        let mistral = profile("openrouter", "mistralai/devstral-small");
        assert_eq!(mistral.system_role, SystemRolePolicy::PrependToFirstUser);
        assert_eq!(mistral.search, SearchPolicy::OpenRouterWebSearchTool);

        let generic =
            profile("openai-compatible", "gbuzhf/KAT-Coder-V2.5-Dev-APEX-MTP-GGUF:I-Mini_local");
        assert_eq!(generic.system_role, SystemRolePolicy::Merged);
    }

    #[test]
    fn openrouter_search_params_use_server_tool_extra_body() {
        let search = profile("openrouter", "deepseek/deepseek-v4-pro").search_params();

        assert!(search.enable_search.is_none());
        assert!(search.search_options.is_none());
        assert_eq!(
            search.extra_body.get("tools"),
            Some(&serde_json::json!([{ "type": "openrouter:web_search" }]))
        );
    }

    #[test]
    fn profile_detection_does_not_match_arbitrary_substrings() {
        assert_eq!(
            OpenAiCompatFamily::from_provider_model("openai-compatible", "notqwen-model"),
            OpenAiCompatFamily::Generic
        );
        assert_eq!(
            OpenAiCompatFamily::from_provider_model("openai-compatible", "made-by-deepseek-ish"),
            OpenAiCompatFamily::Generic
        );
        assert_eq!(
            profile("openai-compatible", "qwen3.80-max").thinking_dialect,
            ThinkingDialect::QwenEnableThinking
        );
    }

    #[test]
    fn thinking_params_preserve_existing_dialects() {
        let qwen = profile("openai-compatible", "qwen3.7-max").thinking_params(Some(Effort::Min));
        assert_eq!(qwen.enable_thinking, Some(false));
        assert!(qwen.reasoning_effort.is_none());

        // Unset effort keeps each dialect's historical default: on.
        let qwen_unset = profile("openai-compatible", "qwen3.7-max").thinking_params(None);
        assert_eq!(qwen_unset.enable_thinking, Some(true));

        let deepseek = profile("deepseek", "deepseek-v4-pro").thinking_params(Some(Effort::Max));
        assert!(matches!(deepseek.reasoning_effort, Some(ReasoningEffort::Max)));
        assert_eq!(deepseek.thinking.unwrap().thinking_type, "enabled");

        let openrouter_deepseek =
            profile("openrouter", "deepseek/deepseek-v4-pro").thinking_params(Some(Effort::Max));
        assert!(matches!(openrouter_deepseek.reasoning_effort, Some(ReasoningEffort::XHigh)));
        assert_eq!(openrouter_deepseek.thinking.unwrap().thinking_type, "enabled");

        let openrouter_deepseek_ga = profile("openrouter", "deepseek/deepseek-v4-pro-0813")
            .thinking_params(Some(Effort::Max));
        assert!(matches!(openrouter_deepseek_ga.reasoning_effort, Some(ReasoningEffort::Max)));
        assert_eq!(openrouter_deepseek_ga.thinking.unwrap().thinking_type, "enabled");

        // The middle of the DeepSeek scale rides on "high", its known-good level.
        let deepseek_mid =
            profile("deepseek", "deepseek-v4-pro").thinking_params(Some(Effort::High));
        assert!(matches!(deepseek_mid.reasoning_effort, Some(ReasoningEffort::High)));

        let openrouter_glm =
            profile("openrouter", "z-ai/glm-5.2").thinking_params(Some(Effort::Max));
        assert!(matches!(openrouter_glm.reasoning_effort, Some(ReasoningEffort::XHigh)));
        assert!(openrouter_glm.thinking.is_none());

        let kimi = profile("openai-compatible", "moonshotai/kimi-k2.6")
            .thinking_params(Some(Effort::Medium));
        assert!(kimi.thinking.is_none());

        let kimi_disabled =
            profile("openai-compatible", "moonshotai/kimi-k2.6").thinking_params(Some(Effort::Min));
        assert_eq!(kimi_disabled.thinking.unwrap().thinking_type, "disabled");

        let kimi_k3 =
            profile("openrouter", "moonshotai/kimi-k3").thinking_params(Some(Effort::Min));
        assert_eq!(kimi_k3.reasoning_effort, Some(ReasoningEffort::Max));
        assert!(kimi_k3.thinking.is_none());

        let kimi_k3_unset = profile("openrouter", "moonshotai/kimi-k3").thinking_params(None);
        assert!(kimi_k3_unset.reasoning_effort.is_none());
    }

    /// Golden table for the generic `reasoning_effort` dialect. `medium` and
    /// `high` now map to their same-named wire values (previously they were
    /// shifted one rung down); `xhigh`/`max` cap at `high` for compatibility
    /// with servers that reject higher values.
    #[test]
    fn generic_reasoning_effort_maps_levels_by_name_capped_at_high() {
        let cases = [
            (Effort::Min, None),
            (Effort::Low, Some(ReasoningEffort::Low)),
            (Effort::Medium, Some(ReasoningEffort::Medium)),
            (Effort::High, Some(ReasoningEffort::High)),
            (Effort::XHigh, Some(ReasoningEffort::High)),
            (Effort::Max, Some(ReasoningEffort::High)),
        ];
        for (requested, expected) in cases {
            let params = profile("openai-compatible", "gpt-oss").thinking_params(Some(requested));
            assert_eq!(params.reasoning_effort, expected, "generic {requested}");
        }
        assert!(
            profile("openai-compatible", "gpt-oss")
                .thinking_params(None)
                .reasoning_effort
                .is_none()
        );
    }

    #[test]
    fn qwen38_maps_graded_reasoning_effort_and_preserves_explicit_off() {
        let cases = [
            (Effort::Min, None, Some(false)),
            (Effort::Low, Some(ReasoningEffort::Low), None),
            (Effort::Medium, Some(ReasoningEffort::Medium), None),
            (Effort::High, Some(ReasoningEffort::Medium), None),
            (Effort::XHigh, Some(ReasoningEffort::XHigh), None),
            (Effort::Max, Some(ReasoningEffort::XHigh), None),
        ];
        for (requested, reasoning_effort, enable_thinking) in cases {
            let params =
                profile("openai-compatible", "Qwen3.8-27B-MLX").thinking_params(Some(requested));
            assert_eq!(params.reasoning_effort, reasoning_effort, "Qwen 3.8 {requested}");
            assert_eq!(params.enable_thinking, enable_thinking, "Qwen 3.8 {requested}");
        }

        let unset = profile("openai-compatible", "Qwen3.8-27B-MLX").thinking_params(None);
        assert!(unset.reasoning_effort.is_none());
        assert!(unset.enable_thinking.is_none());

        let openrouter =
            profile("openrouter", "qwen/qwen3.8-max").thinking_params(Some(Effort::Low));
        assert_eq!(openrouter.reasoning_effort, Some(ReasoningEffort::Low));
        assert!(openrouter.enable_thinking.is_none());
    }

    /// Golden table for the OpenRouter dialect. `medium` no longer collapses
    /// into `high`; `min` still sends an explicit "none" while an unset
    /// effort omits the field entirely.
    #[test]
    fn openrouter_dialect_maps_the_full_scale() {
        let cases = [
            (Effort::Min, ReasoningEffort::None),
            (Effort::Low, ReasoningEffort::Low),
            (Effort::Medium, ReasoningEffort::Medium),
            (Effort::High, ReasoningEffort::High),
            (Effort::XHigh, ReasoningEffort::XHigh),
            (Effort::Max, ReasoningEffort::XHigh),
        ];
        for (requested, expected) in cases {
            let params = profile("openrouter", "z-ai/glm-5.2").thinking_params(Some(requested));
            assert_eq!(params.reasoning_effort, Some(expected), "openrouter {requested}");
        }
        assert!(
            profile("openrouter", "z-ai/glm-5.2").thinking_params(None).reasoning_effort.is_none()
        );
    }

    #[test]
    fn muse_spark_maps_its_mandatory_reasoning_scale() {
        let cases = [
            (Effort::Min, ReasoningEffort::Minimal),
            (Effort::Low, ReasoningEffort::Low),
            (Effort::Medium, ReasoningEffort::Medium),
            (Effort::High, ReasoningEffort::High),
            (Effort::XHigh, ReasoningEffort::XHigh),
            (Effort::Max, ReasoningEffort::XHigh),
        ];
        for (requested, expected) in cases {
            let params =
                profile("openrouter", "meta/muse-spark-1.2").thinking_params(Some(requested));
            assert_eq!(params.reasoning_effort, Some(expected), "Muse Spark {requested}");
        }
    }

    #[test]
    fn effort_levels_match_each_dialect() {
        assert_eq!(
            effort_levels("openai-compatible", "qwen3.7-max"),
            vec![Effort::Min, Effort::Low],
        );
        assert_eq!(
            effort_levels("openai-compatible", "Qwen3.8-27B-MLX"),
            vec![Effort::Min, Effort::Low, Effort::Medium, Effort::XHigh],
        );
        assert_eq!(
            effort_levels("deepseek", "deepseek-v4-pro"),
            vec![Effort::Min, Effort::Medium, Effort::Max],
        );
        assert_eq!(
            effort_levels("openrouter", "z-ai/glm-5.2"),
            vec![Effort::Min, Effort::Low, Effort::Medium, Effort::High, Effort::XHigh],
        );
        assert_eq!(
            effort_levels("openrouter", "meta/muse-spark-1.2"),
            vec![Effort::Min, Effort::Low, Effort::Medium, Effort::High, Effort::XHigh],
        );
        assert_eq!(
            effort_levels("openai-compatible", "gpt-oss"),
            vec![Effort::Min, Effort::Low, Effort::Medium, Effort::High],
        );
        // The None dialect ignores effort entirely: no options.
        assert!(effort_levels("openrouter", "minimax/minimax-m3").is_empty());
    }
}
