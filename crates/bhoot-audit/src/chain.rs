use crate::error::AuditError;
use crate::event::{AuditEvent, AuditEventType};
use chrono::{DateTime, Utc};

/// Cryptographically chained append-only audit trail.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AuditChain {
    events: Vec<AuditEvent>,
}

impl AuditChain {
    /// Genesis previous hash constant (32 zeros).
    pub const GENESIS_PREV_HASH: [u8; 32] = [0u8; 32];

    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Number of events recorded in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Retrieve the slice of recorded audit events.
    #[must_use]
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Mutable access to events for tamper simulation testing only.
    #[cfg(test)]
    pub fn events_mut(&mut self) -> &mut [AuditEvent] {
        &mut self.events
    }

    /// Append a new event to the hash chain.
    pub fn record_event(
        &mut self,
        timestamp: DateTime<Utc>,
        event_type: AuditEventType,
        entity_id: impl Into<String>,
        payload_json: impl Into<String>,
    ) -> &AuditEvent {
        let sequence = self.events.len() as u64;
        let prev_hash = match self.events.last() {
            Some(last) => last.hash,
            None => Self::GENESIS_PREV_HASH,
        };

        let entity_id_str = entity_id.into();
        let payload_str = payload_json.into();

        let hash = AuditEvent::compute_hash(
            sequence,
            &timestamp,
            event_type,
            &entity_id_str,
            &payload_str,
            &prev_hash,
        );

        let event = AuditEvent {
            sequence,
            timestamp,
            event_type,
            entity_id: entity_id_str,
            payload_json: payload_str,
            prev_hash,
            hash,
        };

        self.events.push(event);
        self.events.last().expect("just pushed")
    }

    /// Validate the cryptographic continuity and payload integrity of every block.
    ///
    /// # Errors
    /// Returns `AuditError` if:
    /// - Sequence numbers are not monotonically sequential.
    /// - Genesis block does not start with zeros.
    /// - Any block's `prev_hash` does not match the previous block's `hash`.
    /// - Computed hash does not match stored block hash.
    pub fn verify_integrity(&self) -> Result<(), AuditError> {
        if self.events.is_empty() {
            return Ok(());
        }

        let mut expected_prev_hash = Self::GENESIS_PREV_HASH;

        for (idx, event) in self.events.iter().enumerate() {
            let expected_seq = idx as u64;
            if event.sequence != expected_seq {
                return Err(AuditError::SequenceGap {
                    expected: expected_seq,
                    actual: event.sequence,
                });
            }

            if event.prev_hash != expected_prev_hash {
                return Err(AuditError::BrokenLink {
                    sequence: event.sequence,
                });
            }

            let recomputed_hash = AuditEvent::compute_hash(
                event.sequence,
                &event.timestamp,
                event.event_type,
                &event.entity_id,
                &event.payload_json,
                &event.prev_hash,
            );

            if recomputed_hash != event.hash {
                return Err(AuditError::HashMismatch {
                    sequence: event.sequence,
                    expected: format!("{:02x?}", event.hash),
                    actual: format!("{:02x?}", recomputed_hash),
                });
            }

            expected_prev_hash = event.hash;
        }

        Ok(())
    }
}
