use crate::parser::RustParser;
use crate::parser::symbols::{ActorType, MessageKind, SendMethod};
use std::path::Path;

/// Test Kameo actor detection with associated types
#[test]
pub fn test_kameo_actor_detection() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Import the fixture
    use super::fixtures::kameo_patterns::KAMEO_ACTOR;
    
    let result = parser.parse_source(
        KAMEO_ACTOR,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();
    
    // Should detect the Kameo actors
    assert!(!result.actors.is_empty(), "Should detect Kameo actors");
    
    // Check BybitWsSupervisor actor
    let supervisor = result.actors.iter()
        .find(|a| a.name == "BybitWsSupervisor")
        .expect("Should find BybitWsSupervisor actor");
    
    assert_eq!(supervisor.actor_type, ActorType::Kameo, "Should be identified as Kameo actor");
    assert!(!supervisor.is_distributed, "Should not be distributed");
    
    // Check RestAPIActor
    let rest_actor = result.actors.iter()
        .find(|a| a.name == "RestAPIActor")
        .expect("Should find RestAPIActor");
    
    assert_eq!(rest_actor.actor_type, ActorType::Kameo);
    
    // Check OrderWSActor
    let order_actor = result.actors.iter()
        .find(|a| a.name == "OrderWSActor")
        .expect("Should find OrderWSActor");
    
    assert_eq!(order_actor.actor_type, ActorType::Kameo);
}

/// Test Kameo message sending patterns
#[test]
pub fn test_kameo_message_sending() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    use super::fixtures::kameo_patterns::KAMEO_MESSAGE_SENDING;
    
    let result = parser.parse_source(
        KAMEO_MESSAGE_SENDING,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();
    
    // Should detect message sends
    assert!(!result.message_sends.is_empty(), "Should detect message sends");
    
    // Should have tell patterns
    let tell_sends: Vec<_> = result.message_sends.iter()
        .filter(|s| s.send_method == SendMethod::Tell)
        .collect();
    
    assert!(!tell_sends.is_empty(), "Should detect tell() calls");
    
    // Should have ask patterns
    let ask_sends: Vec<_> = result.message_sends.iter()
        .filter(|s| s.send_method == SendMethod::Ask)
        .collect();
    
    assert!(!ask_sends.is_empty(), "Should detect ask() calls");
    
    // Should detect spawn calls
    let spawn_calls: Vec<_> = result.calls.iter()
        .filter(|c| c.callee_name == "spawn")
        .collect();
    
    assert!(!spawn_calls.is_empty(), "Should detect spawn() calls");
}

/// Test remote/distributed Kameo patterns
#[test]
pub fn test_kameo_remote_patterns() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    use super::fixtures::kameo_patterns::KAMEO_REMOTE_ACTOR;
    
    let result = parser.parse_source(
        KAMEO_REMOTE_ACTOR,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();
    
    // Should detect distributed actor
    let distributed_actor = result.actors.iter()
        .find(|a| a.name == "DistributedProcessor")
        .expect("Should find DistributedProcessor");
    
    assert_eq!(distributed_actor.actor_type, ActorType::Distributed, 
              "Should be identified as distributed based on name");
    assert!(distributed_actor.is_distributed);
    
    // Should detect remote message type
    // Note: We'd need to add #[kameo(remote)] detection to the AST walker
    // For now, we just check that the type is detected
    assert!(result.types.iter().any(|t| t.name == "RemoteDataMessage"),
            "Should detect RemoteDataMessage type");
}

/// Test complex Kameo patterns with multiple actors
#[test]
pub fn test_kameo_complex_patterns() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    use super::fixtures::kameo_patterns::KAMEO_COMPLEX_PATTERNS;
    
    let result = parser.parse_source(
        KAMEO_COMPLEX_PATTERNS,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();
    
    // Should detect multiple actors
    assert!(result.actors.len() >= 2, "Should detect at least 2 actors");
    
    let state_manager = result.actors.iter()
        .find(|a| a.name == "StateManager")
        .expect("Should find StateManager");
    
    let notification_actor = result.actors.iter()
        .find(|a| a.name == "NotificationActor")
        .expect("Should find NotificationActor");
    
    assert_eq!(state_manager.actor_type, ActorType::Kameo);
    assert_eq!(notification_actor.actor_type, ActorType::Kameo);
    
    // Should detect message types
    assert!(result.message_types.iter().any(|m| m.name == "StateMessage"),
            "Should detect StateMessage");
    assert!(result.message_types.iter().any(|m| m.name == "NotificationMessage"),
            "Should detect NotificationMessage");
    
    // Should detect handlers
    assert!(result.message_handlers.len() >= 2, 
            "Should detect handlers for both actors");
}