//! Asking the client to approve a tool call, and reading its answer.

use agent_client_protocol::schema::v1::{
    PermissionOption, PermissionOptionKind, RequestPermissionOutcome,
};
use ante_sdk::protocol::ReviewDecision;

const ALLOW_ONCE: &str = "allow_once";
const ALLOW_ALWAYS: &str = "allow_always";
const REJECT: &str = "reject_once";

/// The choices offered for every approval. "Always" means this session
/// only: a click in the editor never writes a permanent rule.
pub fn options() -> Vec<PermissionOption> {
    vec![
        PermissionOption::new(ALLOW_ONCE, "Allow", PermissionOptionKind::AllowOnce),
        PermissionOption::new(
            ALLOW_ALWAYS,
            "Always allow in this session",
            PermissionOptionKind::AllowAlways,
        ),
        PermissionOption::new(REJECT, "Reject", PermissionOptionKind::RejectOnce),
    ]
}

/// What the client's answer means for Ante. `None` when the client cancelled
/// the turn instead of answering.
pub fn decision(outcome: &RequestPermissionOutcome) -> Option<ReviewDecision> {
    match outcome {
        RequestPermissionOutcome::Selected(selected) => Some(match &*selected.option_id.0 {
            ALLOW_ONCE => ReviewDecision::Accept,
            ALLOW_ALWAYS => ReviewDecision::AcceptForSession,
            _ => ReviewDecision::Deny,
        }),
        RequestPermissionOutcome::Cancelled => None,
        _ => Some(ReviewDecision::Deny),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::SelectedPermissionOutcome;

    fn selected(id: &str) -> RequestPermissionOutcome {
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id.to_string()))
    }

    #[test]
    fn every_offered_option_maps_and_always_stays_in_session() {
        let options = options();
        assert_eq!(options.len(), 3);
        assert_eq!(decision(&selected(&options[0].option_id.0)), Some(ReviewDecision::Accept));
        assert_eq!(
            decision(&selected(&options[1].option_id.0)),
            Some(ReviewDecision::AcceptForSession)
        );
        assert_eq!(decision(&selected(&options[2].option_id.0)), Some(ReviewDecision::Deny));
    }

    #[test]
    fn unknown_answers_deny_and_cancellation_is_distinct() {
        assert_eq!(decision(&selected("made_up")), Some(ReviewDecision::Deny));
        assert_eq!(decision(&RequestPermissionOutcome::Cancelled), None);
    }
}
