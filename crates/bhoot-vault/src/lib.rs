pub mod document;
pub mod error;
pub mod mime;
pub mod store;

pub use document::{VaultDocument, VaultIntegrityReport};
pub use error::VaultError;
pub use mime::DocumentMimeType;
pub use store::{VaultStore, MAX_DOCUMENT_SIZE_BYTES};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_pdf_storage_and_retrieval() {
        let mut vault = VaultStore::new();
        let key = [0x77u8; 32];

        // Valid PDF mock with %PDF- magic header
        let mut pdf_data = b"%PDF-1.7\n".to_vec();
        pdf_data.extend_from_slice(b"1 0 obj << /Title (Contract Note Nov 2026) >> endobj");

        let doc_id = vault
            .store_document("contract_note.pdf", &pdf_data, &key, None)
            .expect("storage succeeds");

        let retrieved = vault
            .retrieve_document(&doc_id, &key)
            .expect("retrieval succeeds");

        assert_eq!(retrieved.as_slice(), pdf_data.as_slice());
    }

    #[test]
    fn test_vault_invalid_magic_bytes_rejected() {
        let mut vault = VaultStore::new();
        let key = [0x77u8; 32];

        let malicious_exe = b"MZ\x90\x00\x03\x00\x00\x00";
        let res = vault.store_document("fake.pdf", malicious_exe, &key, None);
        assert!(matches!(res, Err(VaultError::InvalidMimeType(_))));
    }

    #[test]
    fn test_vault_deduplication() {
        let mut vault = VaultStore::new();
        let key = [0x77u8; 32];

        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG magic header
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR
        ];

        let tx1 = bhoot_core::TransactionId::new_v4();
        let tx2 = bhoot_core::TransactionId::new_v4();

        let id1 = vault
            .store_document("receipt1.png", &png_data, &key, Some(tx1))
            .expect("stored");

        // Store exact same file bytes again
        let id2 = vault
            .store_document("receipt2.png", &png_data, &key, Some(tx2))
            .expect("stored");

        assert_eq!(
            id1, id2,
            "Exact content must return deduplicated document ID"
        );
        assert_eq!(vault.len(), 1, "Vault count must remain 1");

        let meta = vault.get_metadata(&id1).unwrap();
        assert_eq!(meta.linked_transaction_ids.len(), 2);
    }

    #[test]
    fn test_vault_integrity_audit_and_export() {
        let mut vault = VaultStore::new();
        let key = [0x55u8; 32];
        let wrong_key = [0x11u8; 32];

        let pdf_data = b"%PDF-1.4\n1 0 obj << /Type /Catalog >> endobj\nxref\n0 1\n0000000000 65535 f\ntrailer << /Root 1 0 R >>\nstartxref\n50\n%%EOF".to_vec();
        let doc_id = vault.store_document("statement.pdf", &pdf_data, &key, None).unwrap();

        // 1. Audit with correct key -> All verified
        let audit = vault.verify_all_documents(&key);
        assert_eq!(audit.total_documents, 1);
        assert_eq!(audit.verified_documents, 1);
        assert_eq!(audit.corrupted_documents, 0);

        // 2. Audit with wrong key -> Flags corrupted
        let audit_wrong = vault.verify_all_documents(&wrong_key);
        assert_eq!(audit_wrong.corrupted_documents, 1);
        assert_eq!(audit_wrong.corrupted_ids, vec![doc_id]);

        // 3. Export to disk file
        let doc = vault.get_metadata(&doc_id).unwrap();
        let temp = tempfile::NamedTempFile::new().unwrap();
        let bytes_written = doc.export_to_file(&key, temp.path()).unwrap();
        assert_eq!(bytes_written, pdf_data.len());

        let read_back = std::fs::read(temp.path()).unwrap();
        assert_eq!(read_back, pdf_data);

        // 4. Delete document
        assert!(vault.delete_document(&doc_id));
        assert_eq!(vault.len(), 0);
        assert_eq!(vault.total_stored_bytes(), 0);
    }
}
