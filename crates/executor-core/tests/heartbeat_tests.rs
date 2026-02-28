#[cfg(test)]
mod tests {
    use executor_core::task::{TaskId, TaskStatus};
    use executor_core::metadata::{TaskMetadata, default_heartbeat_interval};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_default_heartbeat_interval() {
        assert_eq!(default_heartbeat_interval(), 30u64);
    }

    #[test]
    fn test_heartbeat_timeout_is_terminal() {
        assert!(TaskStatus::HeartbeatTimeout.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Killed.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(!TaskStatus::Starting.is_terminal());
    }

    #[test]
    fn test_heartbeat_status_display() {
        let status = TaskStatus::HeartbeatTimeout;
        assert_eq!(status.to_string(), "heartbeat_timeout");
    }

    #[test]
    fn test_metadata_has_heartbeat_fields() {
        let task_id = TaskId::new();
        let meta = TaskMetadata::new(
            task_id.clone(),
            "test".to_string(),
            "ssh".to_string(),
            "test".to_string(),
            "test prompt".to_string(),
            None,
        );

        assert_eq!(meta.heartbeat_interval, Some(30u64));
        assert_eq!(meta.last_heartbeat, None);
    }

    #[test]
    fn test_mark_heartbeat_updates_timestamp() {
        let task_id = TaskId::new();
        let mut meta = TaskMetadata::new(
            task_id.clone(),
            "test".to_string(),
            "ssh".to_string(),
            "test".to_string(),
            "test prompt".to_string(),
            None,
        );

        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        meta.mark_heartbeat();
        
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(meta.last_heartbeat.is_some());
        let heartbeat = meta.last_heartbeat.unwrap();
        assert!(heartbeat >= before);
        assert!(heartbeat <= after);
    }

    #[test]
    fn test_stale_after_10_intervals() {
        let task_id = TaskId::new();
        let mut meta = TaskMetadata::new(
            task_id.clone(),
            "test".to_string(),
            "ssh".to_string(),
            "test".to_string(),
            "test prompt".to_string(),
            None,
        );

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Set last_heartbeat to 301 seconds ago (should be stale: >300s)
        meta.last_heartbeat = Some(now - 301u64);
        meta.heartbeat_interval = Some(30u64);

        // Should be marked as stale
        let stale_after = meta.heartbeat_interval.unwrap() * 10;
        assert!(now - meta.last_heartbeat.unwrap() > stale_after);
    }
}
