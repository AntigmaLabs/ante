use ante_sdk::protocol::{self, Evt};
use clap::ValueEnum;
use crossterm::style::Stylize;
use std::io::{self, Write};

#[derive(Debug, Clone, Default, ValueEnum)]
pub enum OutputFormat {
    Json,
    Human,
    #[default]
    Minimal,
}

pub fn should_print_event(output_format: &OutputFormat, event: &Evt) -> bool {
    match output_format {
        OutputFormat::Json | OutputFormat::Human => {
            !matches!(event, Evt::MessageDelta(_) | Evt::ThinkingDelta(_))
        }
        OutputFormat::Minimal => {
            matches!(event, Evt::AgentMessage(_) | Evt::Info(_) | Evt::Error(_))
        }
    }
}

pub fn print_event(output_format: &OutputFormat, event: &protocol::EventMsg) {
    if !should_print_event(output_format, &event.event) {
        return;
    }
    match output_format {
        OutputFormat::Json => match serde_json::to_string(event) {
            Ok(line) => {
                let _ = writeln!(io::stdout().lock(), "{line}");
            }
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "Failed to encode event as JSON: {error}");
            }
        },
        OutputFormat::Human | OutputFormat::Minimal => print_event_ansi(&event.event),
    }
}

fn print_event_ansi(event: &Evt) {
    let _ = writeln!(io::stdout().lock());
    match event {
        Evt::AgentMessage(message) => {
            let _ = writeln!(io::stdout().lock(), "{} {}", "❖".bold().dark_yellow(), message);
        }
        Evt::Info(message) => {
            let _ = writeln!(
                io::stdout().lock(),
                "{} {}",
                "ℹ️".dim(),
                message.as_str().dim().dark_cyan()
            );
        }
        Evt::Thinking(message) => {
            let _ =
                writeln!(io::stdout().lock(), "{} {}", "💭".dim(), message.as_str().dim().italic());
        }
        Evt::Error(message) => {
            let _ = writeln!(io::stderr().lock(), "{} {}", "✖".bold().dark_red(), message);
        }
        Evt::ToolStart(tool_use) => {
            let _ = writeln!(
                io::stdout().lock(),
                "{} ToolUse(id={}, name={})",
                "🔧".dim(),
                tool_use.id,
                tool_use.name
            );
        }
        Evt::ToolUpdate(update) => {
            let _ = writeln!(
                io::stdout().lock(),
                "{} {} #{} {}",
                "⏳".dim(),
                update.tool_use_id.as_str().dim(),
                update.seq,
                update.message.as_str().dim()
            );
        }
        Evt::ToolEnd(result) => {
            let _ = writeln!(
                io::stdout().lock(),
                "{} {} {} {}",
                "✅".dim(),
                result.tool_use_id.as_str().dim(),
                result.tool_name,
                format!("{:?}", result.status).dim()
            );
        }
        _ => {
            let _ = writeln!(io::stdout().lock(), "{}", format!("{event:?}").dim());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn all_formats_suppress_streaming_deltas() {
        for format in [OutputFormat::Json, OutputFormat::Human, OutputFormat::Minimal] {
            for event in [
                Evt::MessageDelta("partial".to_string()),
                Evt::ThinkingDelta("partial".to_string()),
            ] {
                assert!(!should_print_event(&format, &event), "{format:?}: {event:?}");
            }
        }
    }

    #[test]
    fn minimal_only_prints_agent_messages_info_and_errors() {
        let events = [
            (Evt::AgentMessage("complete".to_string()), true),
            (Evt::Info("notice".to_string()), true),
            (Evt::Error("failure".to_string()), true),
            (Evt::Thinking("thinking".to_string()), false),
            (Evt::UserInput("prompt".to_string()), false),
            (Evt::CompactStart, false),
            (
                Evt::ToolStart(protocol::ToolUse::new("tool-1", "Read", serde_json::json!({}))),
                false,
            ),
            (
                Evt::ToolUpdate(protocol::ToolUpdate {
                    tool_use_id: "tool-1".to_string(),
                    seq: 1,
                    message: "progress".to_string(),
                }),
                false,
            ),
            (
                Evt::ToolEnd(protocol::ToolEnd {
                    tool_use_id: "tool-1".to_string(),
                    tool_name: "Read".to_string(),
                    status: protocol::ToolEndStatus::Completed,
                    result_json: serde_json::json!({}),
                }),
                false,
            ),
            (Evt::UsageUpdate { usage: Default::default(), context: None }, false),
            (Evt::TurnStart { turn_id: protocol::Id::new("turn") }, false),
            (
                Evt::TurnEnd {
                    turn_id: protocol::Id::new("turn"),
                    status: protocol::TurnEndStatus::Completed,
                    steps: 1,
                },
                false,
            ),
        ];
        for (event, minimal) in events {
            assert!(should_print_event(&OutputFormat::Json, &event), "{event:?}");
            assert!(should_print_event(&OutputFormat::Human, &event), "{event:?}");
            assert_eq!(should_print_event(&OutputFormat::Minimal, &event), minimal, "{event:?}");
        }
    }

    #[test]
    fn clap_values_and_default_match_headless() {
        #[derive(Parser)]
        struct Args {
            #[arg(long, default_value = "minimal")]
            output_format: OutputFormat,
        }

        assert!(matches!(OutputFormat::default(), OutputFormat::Minimal));
        assert!(matches!(
            Args::try_parse_from(["gateway"]).unwrap().output_format,
            OutputFormat::Minimal
        ));

        let names: Vec<_> = OutputFormat::value_variants()
            .iter()
            .map(|format| format.to_possible_value().unwrap().get_name().to_string())
            .collect();
        assert_eq!(names, ["json", "human", "minimal"]);

        for name in names {
            let args = Args::try_parse_from(["gateway", "--output-format", &name]).unwrap();
            assert_eq!(args.output_format.to_possible_value().unwrap().get_name(), name);
        }
        for invalid in ["ansi", "JSON", ""] {
            assert!(Args::try_parse_from(["gateway", "--output-format", invalid]).is_err());
        }
    }
}
