pub mod chain;
pub mod error;
pub mod event;

pub use chain::AuditChain;
pub use error::AuditError;
pub use event::{AuditEvent, AuditEventType, AUDIT_BLOCK_DOMAIN_V1};

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

        assert_eq!(chain.len(), 2);
        chain
            .verify_integrity()
            .expect("Chain should be cryptographically continuous");
    }

    #[test]
    fn test_audit_hash_collision_regression_high_01() {
        // REGRESSION TEST FOR HIGH-01:
        // Prior to remediation, un-delimited concatenation caused identical hashes:
        // acc_1 + 23{"balance":100} == acc_123 + {"balance":100}
        let now = Utc::now();
        let prev_hash = [0u8; 32];

        let hash_a = AuditEvent::compute_hash(
            0,
            &now,
            AuditEventType::AccountCreated,
            "acc_1",
            r#"23{"balance":100}"#,
            &prev_hash,
        );

        let hash_b = AuditEvent::compute_hash(
            0,
            &now,
            AuditEventType::AccountCreated,
            "acc_123",
            r#"{"balance":100}"#,
            &prev_hash,
        );

        assert_ne!(
            hash_a, hash_b,
            "CRITICAL REGRESSION: Collision detected between un-delimited field boundaries!"
        );
    }

    #[test]
    fn test_keyed_hmac_tamper_detection_med_04() {
        // REGRESSION TEST FOR MED-04:
        // Keyed HMAC-SHA256 prevents an attacker with offline DB access
        // from recalculating valid block hashes without knowing the derived audit subkey.
        let audit_key = [0x42u8; 32];
        let mut chain = AuditChain::with_key(audit_key);

        chain.record_event(
            Utc::now(),
            AuditEventType::AccountCreated,
            "acc_1",
            r#"{"balance":100}"#,
        );
        chain.record_event(
            Utc::now(),
            AuditEventType::TransactionPosted,
            "tx_1",
            r#"{"amount":500}"#,
        );

        chain.verify_integrity().expect("Keyed chain is valid");

        // Attacker alters payload of block 0 and tries to recalculate with unkeyed SHA-256
        let mut corrupted_chain = chain.clone();
        corrupted_chain.events_mut()[0].payload_json = r#"{"balance":999999}"#.to_string();

        let fake_hash = AuditEvent::compute_hash(
            0,
            &corrupted_chain.events()[0].timestamp,
            corrupted_chain.events()[0].event_type,
            &corrupted_chain.events()[0].entity_id,
            &corrupted_chain.events()[0].payload_json,
            &corrupted_chain.events()[0].prev_hash,
        );
        corrupted_chain.events_mut()[0].hash = fake_hash;

        // Verification must detect the forgery because the HMAC key was not used
        let res = corrupted_chain.verify_integrity();
        assert!(matches!(
            res,
            Err(AuditError::HashMismatch { sequence: 0, .. })
        ));
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
    }
}
