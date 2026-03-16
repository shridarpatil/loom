use serde::{Deserialize, Serialize};

use crate::error::{LoomError, LoomResult};

/// A workflow defines state transitions for a submittable document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub document_type: String,
    pub states: Vec<WorkflowState>,
    pub transitions: Vec<WorkflowTransition>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub state: String,
    pub doc_status: u8, // 0, 1, or 2
    #[serde(default)]
    pub allow_edit: Option<String>, // Role that can edit in this state
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransition {
    pub state: String,      // From state
    pub action: String,     // e.g., "Approve", "Reject"
    pub next_state: String, // To state
    pub allowed: String,    // Role
    #[serde(default)]
    pub condition: Option<String>, // Rhai expression
}

impl Workflow {
    /// Validate that a transition is allowed from the current state.
    pub fn validate_transition(
        &self,
        current_state: &str,
        action: &str,
        user_roles: &[String],
    ) -> LoomResult<&WorkflowTransition> {
        let transition = self
            .transitions
            .iter()
            .find(|t| t.state == current_state && t.action == action)
            .ok_or_else(|| {
                LoomError::Validation(format!(
                    "No transition '{}' from state '{}'",
                    action, current_state
                ))
            })?;

        if !user_roles.iter().any(|r| r == &transition.allowed) {
            return Err(LoomError::PermissionDenied(format!(
                "Role '{}' required for action '{}' in state '{}'",
                transition.allowed, action, current_state
            )));
        }

        Ok(transition)
    }

    /// Get the initial state of the workflow.
    pub fn initial_state(&self) -> Option<&WorkflowState> {
        self.states.first()
    }
}
