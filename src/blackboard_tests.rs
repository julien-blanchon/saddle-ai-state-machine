use super::*;

#[test]
fn typed_access_and_revision_tracking() {
    let key_health = BlackboardKeyId(0);
    let key_alert = BlackboardKeyId(1);
    let mut blackboard = Blackboard::with_capacity(2);

    assert!(!blackboard.changed_since(0));

    blackboard.set(key_health, 12.5_f32).unwrap();
    blackboard.set(key_alert, true).unwrap();

    assert_eq!(blackboard.get_f32(key_health).unwrap(), Some(12.5));
    assert_eq!(blackboard.get_bool(key_alert).unwrap(), Some(true));
    assert!(blackboard.changed_since(0));
    assert_eq!(blackboard.dirty_keys, vec![key_health, key_alert]);
}

#[test]
fn type_mismatch_returns_error() {
    let key = BlackboardKeyId(0);
    let mut blackboard = Blackboard::with_capacity(1);
    blackboard.set(key, 3_i32).unwrap();

    let error = blackboard.get_bool(key).unwrap_err();
    assert!(matches!(
        error,
        BlackboardError::TypeMismatch {
            expected: BlackboardValueType::Bool,
            actual: BlackboardValueType::I32,
        }
    ));
}

#[test]
fn schema_rejects_wrong_value_type_on_set() {
    let schema = vec![BlackboardKeyDefinition {
        id: BlackboardKeyId(0),
        name: "alert".into(),
        value_type: BlackboardValueType::Bool,
        required: false,
        default_value: Some(false.into()),
    }];
    let mut blackboard = Blackboard::from_schema(&schema);

    let error = blackboard.set(BlackboardKeyId(0), 1_i32).unwrap_err();
    assert!(matches!(
        error,
        BlackboardError::TypeMismatch {
            expected: BlackboardValueType::Bool,
            actual: BlackboardValueType::I32,
        }
    ));
}
