use serde_json::Value;

use crate::error::LoomResult;

/// All possible hook events in the document lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    BeforeInsert,
    Validate,
    BeforeSave,
    AfterInsert,
    OnUpdate,
    AfterSave,
    BeforeSubmit,
    OnSubmit,
    BeforeCancel,
    OnCancel,
    OnTrash,
    OnChange,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::BeforeInsert => "before_insert",
            HookEvent::Validate => "validate",
            HookEvent::BeforeSave => "before_save",
            HookEvent::AfterInsert => "after_insert",
            HookEvent::OnUpdate => "on_update",
            HookEvent::AfterSave => "after_save",
            HookEvent::BeforeSubmit => "before_submit",
            HookEvent::OnSubmit => "on_submit",
            HookEvent::BeforeCancel => "before_cancel",
            HookEvent::OnCancel => "on_cancel",
            HookEvent::OnTrash => "on_trash",
            HookEvent::OnChange => "on_change",
        }
    }

    /// Events that fire during an insert operation, in order.
    pub fn insert_events() -> &'static [HookEvent] {
        &[
            HookEvent::BeforeInsert,
            HookEvent::Validate,
            HookEvent::BeforeSave,
            // DB INSERT happens here
            HookEvent::AfterInsert,
            HookEvent::AfterSave,
        ]
    }

    /// Events that fire during an update operation, in order.
    pub fn update_events() -> &'static [HookEvent] {
        &[
            HookEvent::BeforeSave,
            HookEvent::Validate,
            // DB UPDATE happens here
            HookEvent::OnUpdate,
            HookEvent::AfterSave,
        ]
    }

    /// Events that fire during a submit operation, in order.
    pub fn submit_events() -> &'static [HookEvent] {
        &[
            HookEvent::BeforeSubmit,
            HookEvent::Validate,
            // DB UPDATE docstatus=1 happens here
            HookEvent::OnSubmit,
        ]
    }

    /// Events that fire during a cancel operation, in order.
    pub fn cancel_events() -> &'static [HookEvent] {
        &[
            HookEvent::BeforeCancel,
            // DB UPDATE docstatus=2 happens here
            HookEvent::OnCancel,
        ]
    }

    /// Returns true if this event fires before the DB write.
    pub fn is_before_write(&self) -> bool {
        matches!(
            self,
            HookEvent::BeforeInsert
                | HookEvent::Validate
                | HookEvent::BeforeSave
                | HookEvent::BeforeSubmit
                | HookEvent::BeforeCancel
        )
    }
}

/// A hook handler that can be called during document lifecycle events.
/// This trait is implemented by Rhai scripts, WASM plugins, and compiled Rust hooks.
pub trait HookHandler: Send + Sync {
    fn execute(&self, event: HookEvent, doc: &mut Value) -> LoomResult<()>;
    fn source(&self) -> &str; // e.g., "rhai:leave_application.rhai", "wasm:payroll.wasm"
}
