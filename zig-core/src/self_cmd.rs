//! Commands that act on the currently running zig/zag session.
//!
//! The session's process id is resolved from the `ZAG_PROCESS_ID`
//! environment variable injected by `zag-agent` when the process was
//! spawned. These commands are designed to be invoked by agents from
//! inside their own interactive session (e.g. an interactive `consult`
//! step calling `zig self terminate` to hand control back to the
//! workflow engine).

use crate::error::ZigError;

/// Instruction appended to interactive agents' system prompts so they
/// know how to exit the session when they finish. Exposed for tests in
/// `run_tests.rs` and used from `run::build_agent_config`.
pub const INTERACTIVE_SELF_TERMINATE_INSTRUCTION: &str = "\n\n\
    When you are finished with this session, exit cleanly by running \
    `zig self terminate` in your shell. Do not wait for the user to close \
    the session — self-terminate so the workflow can proceed to the next step.";

/// Terminate the current running session (SIGTERM) by delegating to
/// `zag-orch`'s process registry. Resolves `"self"` via
/// `ZAG_PROCESS_ID`; errors if that env var is not set.
pub fn terminate() -> Result<(), ZigError> {
    zag_orch::ps::request_kill("self")
        .map_err(|e| ZigError::Zag(format!("self terminate failed: {e}")))
}
