//! Minimal chat TUI on top of `ante-sdk`: type a prompt, watch the reply
//! stream in, approve tool calls with `y`/`n`.
//!
//! Run with `cargo run` from this directory.
//!
//! Keys: Enter sends, Esc interrupts the running turn, Ctrl+C quits.
//! Model and provider resolve from your `~/.ante/settings.json`.
//!
//! Everything the SDK does is in `main`, `on_key`, and `on_event`; the rest
//! is ratatui drawing a transcript and an input line.

use ante_sdk::{
    Client, ConnectOptions, connect,
    protocol::{
        Evt, Id, Op, ReviewDecision, SessionRequest, ToolDecision, ToolUse, TurnEndStatus,
        TurnPauseReason,
    },
};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Paragraph, Wrap},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One connection to a host: `stdio` spawns `ante serve --stdio` as a
    // child. Unset session fields resolve to the host's defaults.
    let mut client = connect("stdio".parse()?, ConnectOptions::default()).await?;
    client.send(Op::StartSession(SessionRequest::default())).await?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut client).await;
    ratatui::restore();
    // Sends `Shutdown` and waits for `Goodbye`, so the child exits cleanly.
    client.close().await;
    result
}

async fn run(
    terminal: &mut DefaultTerminal,
    client: &mut Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::default();
    let mut keys = EventStream::new();
    loop {
        terminal.draw(|frame| app.draw(frame))?;
        // Two inputs, one loop: events from the host and the terminal. Every
        // terminal event completes the select so a resize still redraws.
        let op = tokio::select! {
            Some(msg) = client.next_event() => { app.on_event(msg.event); None }
            Some(Ok(event)) = keys.next() => match event {
                Event::Key(key) => app.on_key(key),
                _ => None,
            },
            else => break,
        };
        match op {
            Some(Op::Shutdown) => break,
            Some(op) => client.send(op).await?,
            None => {}
        }
    }
    Ok(())
}

/// Tool name and the start of its arguments, for the transcript and the status line.
fn describe(tool: &ToolUse) -> String {
    let args: String = tool.args.to_string().chars().take(80).collect();
    format!("{} {args}", tool.name)
}

enum Entry {
    User(String),
    Agent(String),
    Tool(String),
    Error(String),
}

#[derive(Default)]
struct App {
    log: Vec<Entry>,
    input: String,
    /// A turn is in flight; Enter is ignored until `TurnEnd`.
    busy: bool,
    /// The last log entry is an agent message still receiving deltas.
    streaming: bool,
    /// A `TurnPause` waiting for `y`/`n`: the turn and the tools it asked about.
    pending: Option<(Id, Vec<ToolUse>)>,
}

impl App {
    /// A key press, and the op it turns into, if any.
    fn on_key(&mut self, key: KeyEvent) -> Option<Op> {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Some(Op::Shutdown),
            (_, KeyCode::Esc) => return Some(Op::Interrupt),
            _ => {}
        }
        if let Some((turn_id, tools)) = &self.pending {
            let decision = match key.code {
                KeyCode::Char('y') => ReviewDecision::Accept,
                KeyCode::Char('n') => ReviewDecision::Deny,
                _ => return None,
            };
            // One answer for the whole batch: the same decision for every tool.
            let responses = tools
                .iter()
                .map(|tool| ToolDecision {
                    tool_use_id: tool.id.clone(),
                    decision: decision.clone(),
                    message: None,
                })
                .collect();
            let op = Op::ApprovalResponse { turn_id: *turn_id, responses };
            self.pending = None;
            return Some(op);
        }
        match key.code {
            KeyCode::Enter if !self.busy && !self.input.trim().is_empty() => {
                let text = std::mem::take(&mut self.input);
                self.log.push(Entry::User(text.clone()));
                self.busy = true;
                Some(Op::UserInput(text))
            }
            KeyCode::Backspace => {
                self.input.pop();
                None
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                None
            }
            _ => None,
        }
    }

    /// An event from the host, folded into the transcript.
    fn on_event(&mut self, event: Evt) {
        match event {
            Evt::MessageDelta(delta) => match self.log.last_mut() {
                Some(Entry::Agent(text)) if self.streaming => text.push_str(&delta),
                _ => {
                    self.log.push(Entry::Agent(delta));
                    self.streaming = true;
                }
            },
            // The complete text follows the deltas; it replaces what streamed.
            Evt::AgentMessage(text) => {
                match self.log.last_mut() {
                    Some(Entry::Agent(last)) if self.streaming => *last = text,
                    _ => self.log.push(Entry::Agent(text)),
                }
                self.streaming = false;
            }
            Evt::ToolStart(tool) => self.log.push(Entry::Tool(describe(&tool))),
            Evt::ToolEnd(end) => {
                self.log.push(Entry::Tool(format!("{} → {:?}", end.tool_name, end.status)));
            }
            Evt::TurnPause { turn_id, reason: TurnPauseReason::Approval { tools, .. } } => {
                self.pending = Some((turn_id, tools));
            }
            Evt::TurnEnd { status, .. } => {
                if let TurnEndStatus::Error { headline, .. } = status {
                    self.log.push(Entry::Error(headline));
                }
                self.busy = false;
                self.streaming = false;
                self.pending = None;
            }
            Evt::Error(text) => self.log.push(Entry::Error(text)),
            _ => {}
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let [log_area, status_area, input_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(3)])
                .areas(frame.area());

        let mut text = Text::default();
        for entry in &self.log {
            let (prefix, body, style) = match entry {
                Entry::User(t) => ("> ", t, Style::new().bold()),
                Entry::Agent(t) => ("", t, Style::new()),
                Entry::Tool(t) => ("⚙ ", t, Style::new().dim()),
                Entry::Error(t) => ("✗ ", t, Style::new().red()),
            };
            for (i, line) in body.lines().enumerate() {
                let prefix = if i == 0 { prefix } else { "" };
                text.push_line(Line::styled(format!("{prefix}{line}"), style));
            }
            text.push_line("");
        }
        let log = Paragraph::new(text).wrap(Wrap { trim: false });
        // Keep the newest lines in view.
        let scroll = log.line_count(log_area.width).saturating_sub(log_area.height as usize);
        frame.render_widget(log.scroll((scroll as u16, 0)), log_area);

        let status = match &self.pending {
            Some((_, tools)) => {
                let calls: Vec<String> = tools.iter().map(describe).collect();
                format!("approve [y/n]: {}", calls.join(" · "))
            }
            None if self.busy => "thinking… (Esc to interrupt)".to_string(),
            None => "Enter to send · Ctrl+C to quit".to_string(),
        };
        frame.render_widget(Line::from(status).dim(), status_area);

        // Scroll the input so the cursor stays inside the box on long prompts.
        let inner = input_area.width.saturating_sub(2);
        let len = self.input.chars().count() as u16;
        let scroll = len.saturating_sub(inner.saturating_sub(1));
        frame.render_widget(
            Paragraph::new(self.input.as_str()).scroll((0, scroll)).block(Block::bordered()),
            input_area,
        );
        frame.set_cursor_position((input_area.x + 1 + len - scroll, input_area.y + 1));
    }
}
