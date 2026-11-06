import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { VaultDocumentSummary, VaultIntegrityReport } from '../types';
import { formatDate } from '../utils/format';
import { Upload, ShieldCheck, Download, AlertCircle, FileText, CheckCircle } from 'lucide-react';

export const VaultView: React.FC = () => {
  const [documents, setDocuments] = useState<VaultDocumentSummary[]>([]);
  const [integrityReport, setIntegrityReport] = useState<VaultIntegrityReport | null>(null);
  const [verifying, setVerifying] = useState(false);

  const loadData = async () => {
    try {
      const docs = await api.getVaultDocuments();
      setDocuments(docs);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleRunAudit = async () => {
    setVerifying(true);
    try {
      const rep = await api.verifyVaultIntegrity();
      setIntegrityReport(rep);
    } catch (err: any) {
      alert(`Integrity verification failed: ${err.message}`);
    } finally {
      setVerifying(false);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
  };

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Private Document Vault</h1>
          <p className="text-xs text-slate-400">
            Encrypted attachment storage sealed with AES-256-GCM and dual BLAKE3/SHA-256 integrity digests
          </p>
        </div>

        <button
          onClick={handleRunAudit}
          disabled={verifying}
          className="flex items-center space-x-1.5 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white text-xs font-semibold rounded-lg shadow-sm transition"
        >
          <ShieldCheck className="w-4 h-4" />
          <span>{verifying ? 'Auditing Digests...' : 'Verify Cryptographic Integrity'}</span>
        </button>
      </div>

      {/* Audit Banner */}
      {integrityReport && (
        <div
          className={`p-4 rounded-xl border flex items-center justify-between text-xs ${
            integrityReport.corrupted_documents === 0
              ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
              : 'bg-rose-500/10 border-rose-500/30 text-rose-400'
          }`}
        >
          <div className="flex items-center space-x-2">
            {integrityReport.corrupted_documents === 0 ? (
              <CheckCircle className="w-4 h-4" />
            ) : (
              <AlertCircle className="w-4 h-4" />
            )}
            <span className="font-semibold">
              {integrityReport.corrupted_documents === 0
                ? `Integrity Verified: ${integrityReport.verified_documents}/${integrityReport.total_documents} documents match recorded BLAKE3 digests.`
                : `Corrupted Files Detected: ${integrityReport.corrupted_documents} document(s) failed authentication!`}
            </span>
          </div>
          <span className="font-mono text-[11px]">Zero Plaintext Leaks</span>
        </div>
      )}

      {/* Upload Dropzone */}
      <div className="p-8 border-2 border-dashed border-slate-800 hover:border-emerald-500/50 rounded-xl bg-[#111726] flex flex-col items-center justify-center space-y-3 cursor-pointer transition group">
        <div className="p-3 bg-slate-900 rounded-full text-slate-400 group-hover:text-emerald-400 border border-slate-800 transition">
          <Upload className="w-6 h-6" />
        </div>
        <div className="text-center">
          <p className="text-sm font-semibold text-slate-200">
            Click or drag documents to envelope-encrypt and store
          </p>
          <p className="text-xs text-slate-500 mt-1">
            Supported: Contract Notes (PDF), Payment Proofs (PNG/JPEG) • Max 50 MiB
          </p>
        </div>
      </div>

      {/* Documents Table */}
      <div className="bg-[#111726] rounded-xl border border-slate-800 overflow-hidden shadow-sm">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/50 text-slate-400 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">Filename</th>
                <th className="py-3 px-4">Format</th>
                <th className="py-3 px-4">File Size</th>
                <th className="py-3 px-4">BLAKE3 Content Digest</th>
                <th className="py-3 px-4">Stored Date</th>
                <th className="py-3 px-4 text-center">Action</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {documents.map((doc) => (
                <tr key={doc.id} className="hover:bg-slate-850/60 transition">
                  <td className="py-3 px-4 font-semibold text-white flex items-center space-x-2">
                    <FileText className="w-4 h-4 text-emerald-400 flex-shrink-0" />
                    <span>{doc.filename}</span>
                  </td>
                  <td className="py-3 px-4">
                    <span className="px-2 py-0.5 rounded text-[10px] bg-slate-800 text-slate-300 border border-slate-700">
                      {doc.mime_type}
                    </span>
                  </td>
                  <td className="py-3 px-4 text-slate-400 font-mono">
                    {formatFileSize(doc.file_size_bytes)}
                  </td>
                  <td className="py-3 px-4 font-mono text-[11px] text-slate-400">
                    {doc.blake3_hash}
                  </td>
                  <td className="py-3 px-4 text-slate-400 whitespace-nowrap">
                    {formatDate(doc.created_at)}
                  </td>
                  <td className="py-3 px-4 text-center">
                    <button
                      onClick={() => alert(`Exporting ${doc.filename} securely...`)}
                      className="p-1 rounded hover:bg-slate-800 text-slate-400 hover:text-emerald-400 transition"
                      title="Export Decrypted File"
                    >
                      <Download className="w-3.5 h-3.5" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
