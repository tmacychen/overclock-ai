use overclock_core::task::{Task, TaskStatus};
use overclock_core::recovery::{classify_error, determine_action, ErrorCategory};
use overclock_core::event::{EventBus, OrchestratorEvent};
use uuid::Uuid;

#[tokio::test]
async fn test_task_creation() {
    // Test task creation and initialization
    let task = Task::new(
        "Test task",
        "Test task description",
        "developer",
    );

    assert_eq!(task.title, "Test task".to_string());
    assert_eq!(task.role, "developer".to_string());
    assert_eq!(task.agent_id, None);
    match task.status {
        TaskStatus::Pending => {},
        _ => panic!("Expected Pending status"),
    }
}

#[tokio::test]
async fn test_error_classification() {
    // Test error classification
    let env_error = "No such file or directory: 'test.txt'";
    let dep_error = "Module not found: 'requests'";
    let code_error = "SyntaxError: invalid syntax";
    let infra_error = "Rate limit exceeded";
    let ambig_error = "Please clarify what you want me to do";
    let unknown_error = "An unknown error occurred";

    assert_eq!(classify_error(env_error), ErrorCategory::Environment);
    assert_eq!(classify_error(dep_error), ErrorCategory::Dependency);
    assert_eq!(classify_error(code_error), ErrorCategory::CodeLogic);
    assert_eq!(classify_error(infra_error), ErrorCategory::Infrastructure);
    assert_eq!(classify_error(ambig_error), ErrorCategory::AmbiguousRequirement);
    assert_eq!(classify_error(unknown_error), ErrorCategory::Unknown);
}

#[tokio::test]
async fn test_error_recovery() {
    // Test error recovery actions
    let env_error = ErrorCategory::Environment;
    let dep_error = ErrorCategory::Dependency;
    let code_error = ErrorCategory::CodeLogic;
    let infra_error = ErrorCategory::Infrastructure;
    let ambig_error = ErrorCategory::AmbiguousRequirement;
    let unknown_error = ErrorCategory::Unknown;

    let env_action = determine_action(&env_error, 0, 3);
    let dep_action = determine_action(&dep_error, 0, 3);
    let code_action = determine_action(&code_error, 0, 3);
    let infra_action = determine_action(&infra_error, 0, 3);
    let ambig_action = determine_action(&ambig_error, 0, 3);
    let unknown_action = determine_action(&unknown_error, 0, 3);

    // Environment and dependency errors should trigger init script run
    match env_action {
        overclock_core::recovery::RecoveryAction::RunInitScript => {},
        _ => panic!("Expected RunInitScript for environment error"),
    }

    match dep_action {
        overclock_core::recovery::RecoveryAction::RunInitScript => {},
        _ => panic!("Expected RunInitScript for dependency error"),
    }

    // Code and infrastructure errors should trigger retry
    match code_action {
        overclock_core::recovery::RecoveryAction::Retry { .. } => {},
        _ => panic!("Expected Retry for code error"),
    }

    match infra_action {
        overclock_core::recovery::RecoveryAction::Retry { .. } => {},
        _ => panic!("Expected Retry for infrastructure error"),
    }

    // Ambiguous and unknown errors should block
    match ambig_action {
        overclock_core::recovery::RecoveryAction::Block { .. } => {},
        _ => panic!("Expected Block for ambiguous error"),
    }

    match unknown_action {
        overclock_core::recovery::RecoveryAction::Block { .. } => {},
        _ => panic!("Expected Block for unknown error"),
    }
}

#[tokio::test]
async fn test_event_bus() {
    // Test event bus functionality
    let event_bus = EventBus::new(100);
    let mut receiver = event_bus.subscribe();

    // Create a test event
    let task_id = Uuid::new_v4();
    let event = OrchestratorEvent::TaskCreated {
        task_id: task_id.clone(),
        title: "Test task".to_string(),
        role: "developer".to_string(),
    };

    // Emit the event
    event_bus.emit(event);

    // Receive the event
    let received_event = receiver.recv().await.unwrap();
    // Just verify that we received an event (we can't compare events directly due to missing PartialEq)
    match received_event {
        OrchestratorEvent::TaskCreated { .. } => {},
        _ => panic!("Expected TaskCreated event"),
    }
}
