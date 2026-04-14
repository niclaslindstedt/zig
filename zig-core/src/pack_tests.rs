use super::*;

#[test]
fn pack_creates_valid_zip() {
    let tmp = tempfile::TempDir::new().unwrap();
    let workflow_dir = tmp.path().join("my-workflow");
    std::fs::create_dir(&workflow_dir).unwrap();

    // Create a workflow TOML
    std::fs::write(
        workflow_dir.join("workflow.toml"),
        r#"[workflow]
name = "test-pack"

[[step]]
name = "hello"
prompt = "Say hello"
"#,
    )
    .unwrap();

    // Create a prompt file
    let prompts_dir = workflow_dir.join("prompts");
    std::fs::create_dir(&prompts_dir).unwrap();
    std::fs::write(prompts_dir.join("greeting.md"), "You are a greeter.").unwrap();

    let output_path = tmp.path().join("output.zwfz");
    let result = pack(
        workflow_dir.to_str().unwrap(),
        Some(output_path.to_str().unwrap()),
    );
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.exists());

    // Verify the zip can be parsed back
    let (wf, _source) = parser::parse_workflow(&output).unwrap();
    assert_eq!(wf.workflow.name, "test-pack");
}

#[test]
fn pack_default_output_name() {
    let tmp = tempfile::TempDir::new().unwrap();
    let workflow_dir = tmp.path().join("healthcare");
    std::fs::create_dir(&workflow_dir).unwrap();

    std::fs::write(
        workflow_dir.join("healthcare.toml"),
        r#"[workflow]
name = "Hospital Triage"

[[step]]
name = "triage"
prompt = "Triage"
"#,
    )
    .unwrap();

    // Change to tmp dir so output goes there
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    let result = pack(workflow_dir.to_str().unwrap(), None);
    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.file_name().unwrap(), "hospital-triage.zwfz");
}

#[test]
fn pack_not_a_directory() {
    let tmp = tempfile::TempDir::new().unwrap();
    let file_path = tmp.path().join("not-a-dir.txt");
    std::fs::write(&file_path, "not a directory").unwrap();

    let result = pack(file_path.to_str().unwrap(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not a directory"));
}

#[test]
fn pack_no_workflow_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let workflow_dir = tmp.path().join("empty");
    std::fs::create_dir(&workflow_dir).unwrap();

    std::fs::write(workflow_dir.join("readme.md"), "No workflow here").unwrap();

    let result = pack(workflow_dir.to_str().unwrap(), None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no workflow TOML file")
    );
}

#[test]
fn pack_round_trip_with_role_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let workflow_dir = tmp.path().join("roles-workflow");
    std::fs::create_dir(&workflow_dir).unwrap();

    let prompts_dir = workflow_dir.join("prompts");
    std::fs::create_dir(&prompts_dir).unwrap();

    std::fs::write(
        workflow_dir.join("workflow.toml"),
        r#"[workflow]
name = "roles-pack"

[roles.doctor]
system_prompt_file = "prompts/doctor.md"

[vars.symptoms]
type = "string"
from = "prompt"

[[step]]
name = "examine"
prompt = "Examine: ${symptoms}"
role = "doctor"
"#,
    )
    .unwrap();
    std::fs::write(
        prompts_dir.join("doctor.md"),
        "You are a board-certified physician.",
    )
    .unwrap();

    let output_path = tmp.path().join("roles-pack.zwfz");
    pack(
        workflow_dir.to_str().unwrap(),
        Some(output_path.to_str().unwrap()),
    )
    .unwrap();

    // Parse the zip and verify role prompt file is accessible
    let (wf, source) = parser::parse_workflow(&output_path).unwrap();
    assert_eq!(wf.roles.len(), 1);
    assert_eq!(
        wf.roles["doctor"].system_prompt_file.as_deref(),
        Some("prompts/doctor.md")
    );

    let prompt_content = std::fs::read_to_string(source.dir().join("prompts/doctor.md")).unwrap();
    assert_eq!(prompt_content, "You are a board-certified physician.");
}
