//! Translating Ante tool events into ACP tool calls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use agent_client_protocol::schema::v1::{
    ContentBlock, Diff, SessionUpdate, TextContent, ToolCall, ToolCallContent, ToolCallLocation,
    ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
use ante_sdk::protocol::{ToolEnd, ToolEndStatus, ToolUpdate, ToolUse};
use serde_json::Value;

/// Tool calls the client has been told about, with their kind. A call
/// denied before it ran ends without ever starting, so its end is announced
/// as a call instead of updating one the client never saw.
pub struct Tools {
    /// The session's working directory, which relative tool paths are under.
    cwd: PathBuf,
    announced: HashMap<String, ToolKind>,
}

impl Tools {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd, announced: HashMap::new() }
    }

    pub fn start(&mut self, tool: ToolUse) -> SessionUpdate {
        let kind = kind(&tool.name);
        self.announced.insert(tool.id.clone(), kind);
        let (content, locations) = self.describe(&tool);
        let mut call = ToolCall::new(tool.id.clone(), tool.name.clone())
            .kind(kind)
            .status(ToolCallStatus::InProgress)
            .raw_input(tool.args.clone());
        if let Some(content) = content {
            call = call.content(content);
        }
        if let Some(locations) = locations {
            call = call.locations(locations);
        }
        SessionUpdate::ToolCall(call)
    }

    /// Describe a call awaiting approval so the client can show it with the
    /// permission prompt, before the host has started it.
    pub fn pending(&mut self, tool: &ToolUse) -> ToolCallUpdate {
        let kind = kind(&tool.name);
        self.announced.insert(tool.id.clone(), kind);
        let (content, locations) = self.describe(tool);
        let fields = ToolCallUpdateFields::new()
            .title(tool.name.clone())
            .kind(kind)
            .status(ToolCallStatus::Pending)
            .raw_input(tool.args.clone())
            .content(content)
            .locations(locations);
        ToolCallUpdate::new(tool.id.clone(), fields)
    }

    /// The file a call touches and, for an edit, the change it makes.
    fn describe(
        &self,
        tool: &ToolUse,
    ) -> (Option<Vec<ToolCallContent>>, Option<Vec<ToolCallLocation>>) {
        let Some(path) = tool.args.get("file_path").and_then(Value::as_str) else {
            return (None, None);
        };
        let path = self.cwd.join(path);
        let content = diff(tool, &path).map(|diff| vec![ToolCallContent::Diff(diff)]);
        (content, Some(vec![ToolCallLocation::new(path)]))
    }

    /// A progress line replaces the call's content until it ends. An edit's
    /// content is its diff, which a progress line must not displace.
    pub fn update(&self, update: ToolUpdate) -> Option<SessionUpdate> {
        match self.announced.get(&update.tool_use_id) {
            Some(kind) if *kind != ToolKind::Edit => {
                let fields = ToolCallUpdateFields::new().content(vec![text(update.message)]);
                Some(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(update.tool_use_id, fields)))
            }
            _ => None,
        }
    }

    pub fn end(&mut self, end: ToolEnd) -> SessionUpdate {
        let status = match end.status {
            ToolEndStatus::Completed => ToolCallStatus::Completed,
            _ => ToolCallStatus::Failed,
        };
        let announced = self.announced.remove(&end.tool_use_id);
        // A finished edit keeps its diff; anything else shows what it said.
        let content = (!(announced.unwrap_or_else(|| kind(&end.tool_name)) == ToolKind::Edit
            && status == ToolCallStatus::Completed))
            .then(|| result_text(&end.result_json).map(|line| vec![text(line)]))
            .flatten();
        if announced.is_some() {
            let fields = ToolCallUpdateFields::new()
                .status(status)
                .content(content)
                .raw_output(end.result_json);
            SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(end.tool_use_id, fields))
        } else {
            let mut call = ToolCall::new(end.tool_use_id, end.tool_name.clone())
                .kind(kind(&end.tool_name))
                .status(status)
                .raw_output(end.result_json);
            if let Some(content) = content {
                call = call.content(content);
            }
            SessionUpdate::ToolCall(call)
        }
    }
}

fn kind(name: &str) -> ToolKind {
    match name {
        "Read" | "ViewImage" => ToolKind::Read,
        "Edit" | "Write" => ToolKind::Edit,
        "Glob" | "Grep" => ToolKind::Search,
        "Bash" => ToolKind::Execute,
        "WebFetch" | "WebSearch" => ToolKind::Fetch,
        "TodoWrite" => ToolKind::Think,
        _ => ToolKind::Other,
    }
}

/// The change an Edit or Write is about to make to `path`. Write's previous
/// content is read from disk when the call starts.
fn diff(tool: &ToolUse, path: &Path) -> Option<Diff> {
    let arg = |name: &str| tool.args.get(name).and_then(Value::as_str);
    match tool.name.as_str() {
        "Edit" => Some(Diff::new(path, arg("new_string")?).old_text(arg("old_string")?)),
        "Write" => {
            Some(Diff::new(path, arg("content")?).old_text(std::fs::read_to_string(path).ok()))
        }
        _ => None,
    }
}

/// The line a tool result has to show, when it has one.
fn result_text(result: &Value) -> Option<String> {
    ["text", "message", "summary", "stdout"]
        .into_iter()
        .find_map(|key| result.get(key).and_then(Value::as_str))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
}

fn text(line: String) -> ToolCallContent {
    ToolCallContent::from(ContentBlock::Text(TextContent::new(line)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn edit(id: &str) -> ToolUse {
        ToolUse::new(
            id,
            "Edit",
            json!({ "file_path": "/src/lib.rs", "old_string": "a", "new_string": "b" }),
        )
    }

    fn end(id: &str, name: &str, status: ToolEndStatus, result: Value) -> ToolEnd {
        ToolEnd { tool_use_id: id.into(), tool_name: name.into(), status, result_json: result }
    }

    fn tools() -> Tools {
        Tools::new(PathBuf::from("/work"))
    }

    fn call(update: SessionUpdate) -> ToolCall {
        match update {
            SessionUpdate::ToolCall(call) => call,
            _ => panic!("expected a tool call"),
        }
    }

    fn update_fields(update: SessionUpdate) -> ToolCallUpdateFields {
        match update {
            SessionUpdate::ToolCallUpdate(update) => update.fields,
            _ => panic!("expected a tool call update"),
        }
    }

    #[test]
    fn an_edit_starts_with_its_diff_and_location() {
        let call = call(tools().start(edit("t1")));

        assert_eq!(&*call.tool_call_id.0, "t1");
        assert_eq!(call.kind, ToolKind::Edit);
        assert_eq!(call.status, ToolCallStatus::InProgress);
        assert_eq!(call.locations[0].path, std::path::PathBuf::from("/src/lib.rs"));
        let [ToolCallContent::Diff(diff)] = call.content.as_slice() else {
            panic!("expected one diff");
        };
        assert_eq!((diff.old_text.as_deref(), diff.new_text.as_str()), (Some("a"), "b"));
        assert_eq!(call.raw_input.unwrap()["old_string"], "a");
    }

    #[test]
    fn relative_paths_resolve_against_the_session_cwd() {
        let relative = ToolUse::new(
            "r",
            "Edit",
            json!({ "file_path": "src/lib.rs", "old_string": "a", "new_string": "b" }),
        );
        let call = call(tools().start(relative));
        assert_eq!(call.locations[0].path, PathBuf::from("/work/src/lib.rs"));
        let [ToolCallContent::Diff(diff)] = call.content.as_slice() else {
            panic!("expected one diff");
        };
        assert_eq!(diff.path, PathBuf::from("/work/src/lib.rs"));
    }

    #[test]
    fn progress_never_displaces_an_edits_diff() {
        let mut tools = tools();
        tools.start(edit("t1"));
        let progress = ToolUpdate { tool_use_id: "t1".into(), seq: 0, message: "writing".into() };
        assert!(tools.update(progress).is_none());
    }

    #[test]
    fn a_pending_call_is_described_for_the_permission_prompt() {
        let mut tools = tools();
        let pending = tools.pending(&edit("t1"));
        assert_eq!(&*pending.tool_call_id.0, "t1");
        assert_eq!(pending.fields.status, Some(ToolCallStatus::Pending));
        assert_eq!(pending.fields.kind, Some(ToolKind::Edit));
        assert_eq!(pending.fields.title.as_deref(), Some("Edit"));
        assert_eq!(pending.fields.locations.as_ref().map(Vec::len), Some(1));
        assert!(matches!(pending.fields.content.as_deref(), Some([ToolCallContent::Diff(_)])));

        // The client has seen it: a later denial updates it instead of announcing it.
        let denied = json!({ "text": "Tool call denied by user and was not executed." });
        let fields = update_fields(tools.end(end("t1", "Edit", ToolEndStatus::Denied, denied)));
        assert_eq!(fields.status, Some(ToolCallStatus::Failed));
    }

    #[test]
    fn a_write_diffs_against_the_file_on_disk_or_nothing() {
        let dir = std::env::temp_dir().join(format!("ante-acp-tools-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let existing = dir.join("notes.md");
        std::fs::write(&existing, "old").unwrap();

        let write = |path: &std::path::Path| {
            ToolUse::new("w", "Write", json!({ "file_path": path, "content": "new" }))
        };
        let overwrite = call(tools().start(write(&existing)));
        let [ToolCallContent::Diff(diff)] = overwrite.content.as_slice() else {
            panic!("expected one diff");
        };
        assert_eq!(diff.old_text.as_deref(), Some("old"));

        let create = call(tools().start(write(&dir.join("missing.md"))));
        let [ToolCallContent::Diff(diff)] = create.content.as_slice() else {
            panic!("expected one diff");
        };
        assert_eq!(diff.old_text, None);
    }

    #[test]
    fn a_command_reports_progress_and_its_output() {
        let mut tools = tools();
        let bash = ToolUse::new("b", "Bash", json!({ "command": "ls" }));
        let call = call(tools.start(bash));
        assert_eq!(call.kind, ToolKind::Execute);
        assert!(call.content.is_empty() && call.locations.is_empty());

        let progress = ToolUpdate { tool_use_id: "b".into(), seq: 0, message: "running".into() };
        let fields = update_fields(tools.update(progress).expect("known call"));
        assert_eq!(fields.content.unwrap().len(), 1);

        let result = json!({ "stdout": "a.rs\n", "stderr": "", "exit_code": 0 });
        let fields =
            update_fields(tools.end(end("b", "Bash", ToolEndStatus::Completed, result.clone())));
        assert_eq!(fields.status, Some(ToolCallStatus::Completed));
        assert_eq!(fields.raw_output, Some(result));
        let Some([ToolCallContent::Content(content)]) = fields.content.as_deref() else {
            panic!("expected the output as content");
        };
        assert_eq!(content.content, ContentBlock::Text(TextContent::new("a.rs\n")));
    }

    #[test]
    fn a_finished_edit_keeps_its_diff() {
        let mut tools = tools();
        tools.start(edit("t1"));
        let result = json!({ "summary": "File edited successfully.", "patch": {} });
        let fields = update_fields(tools.end(end("t1", "Edit", ToolEndStatus::Completed, result)));
        assert_eq!(fields.status, Some(ToolCallStatus::Completed));
        assert_eq!(fields.content, None);

        tools.start(edit("t2"));
        let failed = json!({ "text": "old_string not found" });
        let fields = update_fields(tools.end(end("t2", "Edit", ToolEndStatus::Failed, failed)));
        assert_eq!(fields.status, Some(ToolCallStatus::Failed));
        assert_eq!(fields.content.unwrap().len(), 1);
    }

    #[test]
    fn a_call_denied_before_starting_is_announced_as_failed() {
        let denied = json!({ "text": "Tool call denied by policy and was not executed." });
        let call = call(tools().end(end("d", "Bash", ToolEndStatus::Denied, denied)));
        assert_eq!(call.kind, ToolKind::Execute);
        assert_eq!(call.status, ToolCallStatus::Failed);
        assert_eq!(call.content.len(), 1);
    }

    #[test]
    fn unknown_progress_is_dropped_and_kinds_map() {
        let progress = ToolUpdate { tool_use_id: "nope".into(), seq: 0, message: "x".into() };
        assert!(tools().update(progress).is_none());
        assert_eq!(kind("Grep"), ToolKind::Search);
        assert_eq!(kind("WebFetch"), ToolKind::Fetch);
        assert_eq!(kind("mcp__github__list_issues"), ToolKind::Other);
    }
}
