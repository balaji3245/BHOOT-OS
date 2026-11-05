pub mod chain;
pub mod error;
pub mod event;

pub use chain::AuditChain;
pub use error::AuditError;
pub use event::{AuditEvent, AuditEventType};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_audit_chain_creation_and_continuity() {
        let mut chain = AuditChain::new();
        assert_eq!(chain.len(), 0);

        let t1 = Utc::now();
        chain.record_event(
            t1,
            AuditEventType::AccountCreated,
            "acc_123",
            r#"{"name":"HDFC Bank","type":"Asset"}"#,
        );

        let t2 = Utc::now();
        chain.record_event(
            t2,
            AuditEventType::TransactionPosted,
            "tx_456",
            r#"{"amount_paisa":5000000,"type":"Deposit"}"#,
        );

        let t3 = Utc::now();
        chain.record_event(
            t3,
            AuditEventType::SystemLocked,
            "sys_0",
            r#"{"reason":"Auto-timeout"}"#,
        );

        assert_eq!(chain.len(), 3);
        chain
            .verify_integrity()
            .expect("Chain should be cryptographically continuous");
    }

    #[test]
    fn test_audit_chain_tamper_detection() {
        let mut chain = AuditChain::new();
        let t1 = Utc::now();
        chain.record_event(t1, AuditEventType::AccountCreated, "acc_1", "payload1");
        chain.record_event(
            Utc::now(),
            AuditEventType::TransactionPosted,
            "tx_1",
            "payload2",
        );

        chain.verify_integrity().expect("Unmodified chain is valid");

        // Simulate attacker tampering with historical payload in SQLite
        let mut corrupted_chain = chain.clone();
        corrupted_chain.events_mut()[0].payload_json = "attacker_modified_payload".to_string();

        let err = corrupted_chain.verify_integrity();
        assert!(matches!(
            err,
            Err(AuditError::HashMismatch { sequence: 0, .. })
        ));

        // Simulate attacker recomputing block 0 hash, which breaks block 1's link
        let recomputed_block0_hash = AuditEvent::compute_hash(
            0,
            &corrupted_chain.events()[0].timestamp,
            corrupted_chain.events()[0].event_type,
            &corrupted_chain.events()[0].entity_id,
            &corrupted_chain.events()[0].payload_json,
            &corrupted_chain.events()[0].prev_hash,
        );
        corrupted_chain.events_mut()[0].hash = recomputed_block0_hash;

        let err2 = corrupted_chain.verify_integrity();
        assert!(matches!(err2, Err(AuditError::BrokenLink { sequence: 1 })));
    }
}
