use axum::Json;
use axum::extract::Path;
use serde::{Deserialize, Serialize};

use crate::error::ServeError;

#[cfg(test)]
#[path = "workflows_tests.rs"]
mod tests;

/// Reject workflow names that include path separators, traversal segments,
/// absolute-path markers, or control characters. API callers must use a
/// plain workflow name — filesystem paths are CLI-only.
fn validate_workflow_name(name: &str) -> Result<(), ServeError> {
    if name.is_empty() {
        return Err(ServeError::bad_request("workflow name must not be empty"));
    }
    if name.len() > 255 {
        return Err(ServeError::bad_request("workflow name is too long"));
    }
    if name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name == "."
        || name == ".."
        || name.starts_with('-')
        || name.chars().any(|c| c.is_control())
    {
        return Err(ServeError::bad_request(
            "workflow name must not contain path separators, traversal segments, or control characters",
        ));
    }
    Ok(())
}

// -- List workflows --

pub async fn list() -> Result<Json<Vec<zig_core::manage::WorkflowInfo>>, ServeError> {
    let infos = tokio::task::spawn_blocking(zig_core::manage::get_workflow_list)
        .await
        .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
        .map_err(ServeError::from)?;
    Ok(Json(infos))
}

// -- Show workflow --

pub async fn show(
    Path(name): Path<String>,
) -> Result<Json<zig_core::workflow::model::Workflow>, ServeError> {
    validate_workflow_name(&name)?;
    let workflow =
        tokio::task::spawn_blocking(move || zig_core::manage::get_workflow_detail(&name))
            .await
            .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
            .map_err(ServeError::from)?;
    Ok(Json(workflow))
}

// -- Delete workflow --

pub async fn delete(Path(name): Path<String>) -> Result<axum::http::StatusCode, ServeError> {
    validate_workflow_name(&name)?;
    tokio::task::spawn_blocking(move || zig_core::manage::delete_workflow(&name))
        .await
        .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
        .map_err(ServeError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// -- Validate workflow --

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub content: String,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_count: Option<usize>,
}

pub async fn validate(
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, ServeError> {
    let result: Result<ValidateResponse, zig_core::error::ZigError> =
        tokio::task::spawn_blocking(move || {
            let workflow = zig_core::workflow::parser::parse(&req.content)?;

            match zig_core::workflow::validate::validate(&workflow) {
                Ok(()) => Ok(ValidateResponse {
                    valid: true,
                    errors: vec![],
                    name: Some(workflow.workflow.name),
                    step_count: Some(workflow.steps.len()),
                }),
                Err(errors) => Ok(ValidateResponse {
                    valid: false,
                    errors: errors.iter().map(|e| e.to_string()).collect(),
                    name: Some(workflow.workflow.name),
                    step_count: Some(workflow.steps.len()),
                }),
            }
        })
        .await
        .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?;
    Ok(Json(result.map_err(ServeError::from)?))
}

// -- Run workflow --

#[derive(Deserialize)]
pub struct RunRequest {
    pub workflow: String,
    pub prompt: Option<String>,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub zig_session_id: String,
}

pub async fn run(Json(req): Json<RunRequest>) -> Result<Json<RunResponse>, ServeError> {
    validate_workflow_name(&req.workflow)?;
    // We need to run the workflow and capture the session ID.
    // The current run_workflow doesn't return it, so we'll read the session
    // index before/after to find the new session.
    let sessions_before: Vec<String> = zig_core::session::list_sessions()
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.zig_session_id)
        .collect();

    let workflow = req.workflow.clone();
    let prompt = req.prompt.clone();

    // Spawn the workflow execution on a blocking thread (fire-and-forget)
    tokio::task::spawn_blocking(move || {
        if let Err(e) =
            zig_core::run::run_workflow(&workflow, prompt.as_deref(), false, false, false)
        {
            tracing::error!("workflow execution failed: {e}");
        }
    });

    // Give the session a moment to register in the index
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Find the newly created session
    let sessions_after = zig_core::session::list_sessions().unwrap_or_default();
    let new_session = sessions_after
        .into_iter()
        .find(|s| !sessions_before.contains(&s.zig_session_id));

    match new_session {
        Some(session) => Ok(Json(RunResponse {
            zig_session_id: session.zig_session_id,
        })),
        None => {
            // If we can't find it yet, return the latest session as best effort
            let sessions = zig_core::session::list_sessions().unwrap_or_default();
            match sessions.last() {
                Some(session) => Ok(Json(RunResponse {
                    zig_session_id: session.zig_session_id.clone(),
                })),
                None => Err(ServeError::bad_request(
                    "workflow started but session ID could not be determined",
                )),
            }
        }
    }
}

// -- Create workflow --

#[derive(Deserialize)]
pub struct CreateRequest {
    pub name: Option<String>,
    pub output: Option<String>,
    pub pattern: Option<String>,
}

pub async fn create(
    Json(req): Json<CreateRequest>,
) -> Result<Json<zig_core::create::CreateParams>, ServeError> {
    let params = tokio::task::spawn_blocking(move || {
        zig_core::create::prepare_create(
            req.name.as_deref(),
            req.output.as_deref(),
            req.pattern.as_deref(),
        )
    })
    .await
    .map_err(|e| ServeError::bad_request(format!("task join error: {e}")))?
    .map_err(ServeError::from)?;
    Ok(Json(params))
}
