import React, { useEffect, useState, useRef } from 'react';
import { api } from '../api/client';
import { VaultDocumentSummary, VaultIntegrityReport } from '../types';
import { formatDate } from '../utils/format';
import {
  Upload,
  ShieldCheck,
  Download,
  AlertCircle,
  FileText,
  CheckCircle,
  Trash2,
  Loader2,
} from 'lucide-react';

export const VaultView: React.FC = () => {
  const [documents, setDocuments] = useState<VaultDocumentSummary[]>([]);
  const [integrityReport, setIntegrityReport] = useState<VaultIntegrityReport | null>(null);
  const [verifying, setVerifying] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [uploadSuccess, setUploadSuccess] = useState<string | null>(null);

  const fileInputRef = useRef<HTMLInputElement>(null);

  const loadData = async () => {
    try {
      const docs = await api.getVaultDocuments();
      setDocuments(docs);
    } catch (err: any) {
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

  const processFile = async (file: File) => {
    if (file.size > 50 * 1024 * 1024) {
      setUploadError('File exceeds 50 MiB size limit');
      return;
    }

    setUploading(true);
    setUploadError(null);
    setUploadSuccess(null);

    const reader = new FileReader();
    reader.onload = async () => {
      try {
        const result = reader.result as string;
        // Strip data URI header: "data:application/pdf;base64,"
        const base64Data = result.split(',')[1] || result;
        await api.uploadVaultDocument(file.name, base64Data);
        setUploadSuccess(`Successfully encrypted and sealed "${file.name}"`);
        await loadData();
      } catch (err: any) {
        setUploadError(err.message || 'Failed to upload document');
      } finally {
        setUploading(false);
      }
    };

    reader.onerror = () => {
      setUploadError('Failed to read local file');
      setUploading(false);
    };

    reader.readAsDataURL(file);
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      processFile(e.target.files[0]);
    }
  };

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === 'dragenter' || e.type === 'dragover') {
      setDragActive(true);
    } else if (e.type === 'dragleave') {
      setDragActive(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      processFile(e.dataTransfer.files[0]);
    }
  };

  const handleExport = async (doc: VaultDocumentSummary) => {
    try {
      const result = await api.downloadVaultDocument(doc.id);
      const byteChars = atob(result.data_base64);
      const byteNumbers = new Array(byteChars.length);
      for (let i = 0; i < byteChars.length; i++) {
        byteNumbers[i] = byteChars.charCodeAt(i);
      }
      const byteArray = new Uint8Array(byteNumbers);
      const mime =
        result.mime_type === 'Pdf'
          ? 'application/pdf'
          : result.mime_type === 'Png'
          ? 'image/png'
          : 'image/jpeg';
      const blob = new Blob([byteArray], { type: mime });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = doc.filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    } catch (err: any) {
      alert(`Export failed: ${err.message}`);
    }
  };

  const handleDelete = async (id: string, filename: string) => {
    if (!confirm(`Permanently shred and delete "${filename}" from encrypted vault?`)) {
      return;
    }
    try {
      await api.deleteVaultDocument(id);
      await loadData();
    } catch (err: any) {
      alert(`Delete failed: ${err.message}`);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
  };

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      {/* Hidden native file input */}
      <input
        ref={fileInputRef}
        type="file"
        accept=".pdf,.png,.jpg,.jpeg"
        onChange={handleFileChange}
        className="hidden"
      />

      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Private Document Vault</h1>
          <p className="text-xs text-neutral-400">
            Encrypted attachment storage sealed with AES-256-GCM and dual BLAKE3/SHA-256 integrity digests
          </p>
        </div>

        <button
          onClick={handleRunAudit}
          disabled={verifying}
          className="flex items-center space-x-2 px-3.5 py-2 bg-white hover:bg-neutral-200 disabled:opacity-50 text-black text-xs font-semibold rounded-lg shadow transition"
        >
          <ShieldCheck className="w-4 h-4" />
          <span>{verifying ? 'Auditing Digests...' : 'Verify Vault Integrity'}</span>
        </button>
      </div>

      {/* Audit Banner */}
      {integrityReport && (
        <div
          className={`p-4 rounded-xl border flex items-center justify-between text-xs ${
            integrityReport.corrupted_documents === 0
              ? 'bg-[#111111] border-neutral-700 text-white'
              : 'bg-red-950/40 border-red-800 text-red-300'
          }`}
        >
          <div className="flex items-center space-x-2">
            {integrityReport.corrupted_documents === 0 ? (
              <CheckCircle className="w-4 h-4 text-emerald-400" />
            ) : (
              <AlertCircle className="w-4 h-4 text-red-400" />
            )}
            <span className="font-semibold">
              {integrityReport.corrupted_documents === 0
                ? `Cryptographic Audit Passed: ${integrityReport.verified_documents}/${integrityReport.total_documents} documents match recorded digests.`
                : `Corrupted Files Detected: ${integrityReport.corrupted_documents} document(s) failed authentication!`}
            </span>
          </div>
          <span className="font-mono text-[11px] text-neutral-400">Zero Plaintext Leaks</span>
        </div>
      )}

      {/* Error or Success notification */}
      {uploadError && (
        <div className="p-3 bg-red-950/40 border border-red-800 rounded-lg flex items-center space-x-2 text-xs text-red-300">
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          <span>{uploadError}</span>
        </div>
      )}
      {uploadSuccess && (
        <div className="p-3 bg-neutral-900 border border-neutral-700 rounded-lg flex items-center space-x-2 text-xs text-neutral-200">
          <CheckCircle className="w-4 h-4 flex-shrink-0 text-emerald-400" />
          <span>{uploadSuccess}</span>
        </div>
      )}

      {/* Upload Dropzone */}
      <div
        onClick={() => fileInputRef.current?.click()}
        onDragEnter={handleDrag}
        onDragOver={handleDrag}
        onDragLeave={handleDrag}
        onDrop={handleDrop}
        className={`p-8 border-2 border-dashed rounded-xl bg-[#111111] flex flex-col items-center justify-center space-y-3 cursor-pointer transition ${
          dragActive
            ? 'border-white bg-[#1a1a1a]'
            : 'border-neutral-800 hover:border-neutral-600'
        }`}
      >
        <div className="p-3 bg-neutral-900 rounded-full text-white border border-neutral-800">
          {uploading ? (
            <Loader2 className="w-6 h-6 animate-spin text-white" />
          ) : (
            <Upload className="w-6 h-6" />
          )}
        </div>
        <div className="text-center">
          <p className="text-sm font-semibold text-neutral-200">
            {uploading ? 'Envelope Encrypting & Storing...' : 'Click or drag files to envelope-encrypt and store'}
          </p>
          <p className="text-xs text-neutral-500 mt-1">
            Supported: Contract Notes (PDF), Proofs (PNG/JPEG) • Maximum 50 MiB
          </p>
        </div>
      </div>

      {/* Documents Table */}
      <div className="bg-[#111111] rounded-xl border border-neutral-800 overflow-hidden shadow">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-neutral-800 bg-[#0d0d0d] text-neutral-400 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">Filename</th>
                <th className="py-3 px-4">Format</th>
                <th className="py-3 px-4">File Size</th>
                <th className="py-3 px-4">BLAKE3 Content Digest</th>
                <th className="py-3 px-4">Stored Date</th>
                <th className="py-3 px-4 text-center">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-neutral-800/80">
              {documents.length === 0 ? (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-neutral-500">
                    No documents currently stored in encrypted vault.
                  </td>
                </tr>
              ) : (
                documents.map((doc) => (
                  <tr key={doc.id} className="hover:bg-[#161616] transition">
                    <td className="py-3 px-4 font-semibold text-white flex items-center space-x-2">
                      <FileText className="w-4 h-4 text-neutral-400 flex-shrink-0" />
                      <span>{doc.filename}</span>
                    </td>
                    <td className="py-3 px-4">
                      <span className="px-2 py-0.5 rounded text-[10px] bg-neutral-900 text-neutral-300 border border-neutral-800 font-mono">
                        {doc.mime_type}
                      </span>
                    </td>
                    <td className="py-3 px-4 text-neutral-400 font-mono">
                      {formatFileSize(doc.file_size_bytes)}
                    </td>
                    <td className="py-3 px-4 font-mono text-[11px] text-neutral-400 truncate max-w-xs" title={doc.blake3_hash}>
                      {doc.blake3_hash}
                    </td>
                    <td className="py-3 px-4 text-neutral-400 whitespace-nowrap">
                      {formatDate(doc.created_at)}
                    </td>
                    <td className="py-3 px-4 text-center">
                      <div className="flex items-center justify-center space-x-2">
                        <button
                          onClick={() => handleExport(doc)}
                          className="p-1.5 rounded hover:bg-neutral-800 text-neutral-400 hover:text-white transition"
                          title="Export Decrypted File"
                        >
                          <Download className="w-3.5 h-3.5" />
                        </button>
                        <button
                          onClick={() => handleDelete(doc.id, doc.filename)}
                          className="p-1.5 rounded hover:bg-red-950/40 text-neutral-400 hover:text-red-400 transition"
                          title="Permanently Delete"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
