import React, { useState } from 'react';
import { api } from '../api/client';
import { HardDriveDownload, RefreshCw, CheckCircle2, AlertTriangle, ShieldCheck } from 'lucide-react';

export const BackupRecoveryView: React.FC = () => {
  const [backupPassphrase, setBackupPassphrase] = useState('');
  const [confirmPassphrase, setConfirmPassphrase] = useState('');
  const [backupPath, setBackupPath] = useState('/var/bhoot/backups/bhoot_backup.bhoot');
  const [backingUp, setBackingUp] = useState(false);
  const [backupSuccess, setBackupSuccess] = useState<string | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);

  // Restore state
  const [restoreFile, setRestoreFile] = useState('/var/bhoot/backups/bhoot_backup.bhoot');
  const [restorePassphrase, setRestorePassphrase] = useState('');
  const [restoring, setRestoring] = useState(false);
  const [restoreSuccess, setRestoreSuccess] = useState<string | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);

  const handleCreateBackup = async (e: React.FormEvent) => {
    e.preventDefault();
    setBackupSuccess(null);
    setBackupError(null);

    if (!backupPassphrase || backupPassphrase !== confirmPassphrase) {
      setBackupError('Passphrases do not match or are empty');
      return;
    }
    if (backupPassphrase.length < 12) {
      setBackupError('Backup passphrase must be at least 12 characters');
      return;
    }

    setBackingUp(true);

    try {
      const res = await api.createBackup(backupPassphrase, backupPath);
      setBackupSuccess(
        `Encrypted archive created successfully: ${(res.bytes_written / 1024).toFixed(1)} KB written to ${res.path}. WAL checkpoint flushed and vault documents packed.`
      );
      setBackupPassphrase('');
      setConfirmPassphrase('');
    } catch (err: any) {
      setBackupError(`Backup creation failed: ${err.message}`);
    } finally {
      setBackingUp(false);
    }
  };

  const handleRestoreTest = async (e: React.FormEvent) => {
    e.preventDefault();
    setRestoreSuccess(null);
    setRestoreError(null);

    if (!restorePassphrase) {
      setRestoreError('Enter backup passphrase');
      return;
    }
    if (!restoreFile.trim()) {
      setRestoreError('Specify archive path to restore');
      return;
    }

    setRestoring(true);

    try {
      const res = await api.restoreBackup(restorePassphrase, restoreFile.trim());
      if (res.success) {
        setRestoreSuccess(
          `Archive restored and integrity verified! Restored ${(res.restored_bytes / 1024).toFixed(1)} KB. Database and vault synchronization intact.`
        );
        setRestorePassphrase('');
      } else {
        setRestoreError('Restoration could not be verified.');
      }
    } catch (err: any) {
      setRestoreError(`Restoration verification failed: ${err.message}`);
    } finally {
      setRestoring(false);
    }
  };

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div>
        <h1 className="text-2xl font-extrabold text-white tracking-tight">
          Backup & Disaster Recovery
        </h1>
        <p className="text-xs text-neutral-400">
          Argon2id and AES-256-GCM encrypted airgapped archives with automated WAL flush and restoration verification
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Create Encrypted Backup */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] space-y-5">
          <div className="flex items-center space-x-3 text-white border-b border-[#222222] pb-3">
            <HardDriveDownload className="w-5 h-5 text-white" />
            <h3 className="text-base font-bold text-white">Create Encrypted Backup Archive</h3>
          </div>

          <p className="text-xs text-neutral-400 leading-relaxed">
            Flushes WAL database checkpoint to disk, bundles SQLCipher ledger and encrypted vault documents,
            and seals the payload using AES-256-GCM with a key derived via Argon2id (1 GiB RAM / 4 threads).
          </p>

          {backupSuccess && (
            <div className="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-lg flex items-center space-x-2 text-xs text-emerald-400">
              <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
              <span>{backupSuccess}</span>
            </div>
          )}

          {backupError && (
            <div className="p-3 bg-red-500/10 border border-red-500/30 rounded-lg flex items-center space-x-2 text-xs text-red-400">
              <AlertTriangle className="w-4 h-4 flex-shrink-0" />
              <span>{backupError}</span>
            </div>
          )}

          <form onSubmit={handleCreateBackup} className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-neutral-300 mb-1">
                Destination Archive Path
              </label>
              <input
                type="text"
                value={backupPath}
                onChange={(e) => setBackupPath(e.target.value)}
                className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white font-mono focus:border-white focus:outline-none"
                required
              />
              <span className="text-[10px] text-neutral-500 block mt-1">
                Path must target offline persistent storage (path traversal and system paths /dev, /etc are blocked)
              </span>
            </div>

            <div>
              <label className="block text-xs font-semibold text-neutral-300 mb-1">
                New Backup Passphrase (minimum 12 characters)
              </label>
              <input
                type="password"
                value={backupPassphrase}
                onChange={(e) => setBackupPassphrase(e.target.value)}
                className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none font-mono"
                placeholder="••••••••••••••••"
                required
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-neutral-300 mb-1">
                Confirm Backup Passphrase
              </label>
              <input
                type="password"
                value={confirmPassphrase}
                onChange={(e) => setConfirmPassphrase(e.target.value)}
                className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none font-mono"
                placeholder="••••••••••••••••"
                required
              />
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={backingUp}
                className="w-full py-2.5 bg-white hover:bg-neutral-200 disabled:opacity-50 text-black text-xs font-semibold rounded-lg shadow-sm transition"
              >
                {backingUp ? 'Deriving Key & Encrypting Archive...' : 'Generate Encrypted Backup'}
              </button>
            </div>
          </form>
        </div>

        {/* Restore & Verify */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] space-y-5">
          <div className="flex items-center space-x-3 text-white border-b border-[#222222] pb-3">
            <RefreshCw className="w-5 h-5 text-white" />
            <h3 className="text-base font-bold text-white">Disaster Recovery & Restoration</h3>
          </div>

          <p className="text-xs text-neutral-400 leading-relaxed">
            Performs cryptographic verification and decodes the sealed backup archive. Verifies the SHA-256
            header checksum, decrypts database and documents, and updates application state.
          </p>

          {restoreSuccess && (
            <div className="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-lg flex items-center space-x-2 text-xs text-emerald-400">
              <ShieldCheck className="w-4 h-4 flex-shrink-0" />
              <span>{restoreSuccess}</span>
            </div>
          )}

          {restoreError && (
            <div className="p-3 bg-red-500/10 border border-red-500/30 rounded-lg flex items-center space-x-2 text-xs text-red-400">
              <AlertTriangle className="w-4 h-4 flex-shrink-0" />
              <span>{restoreError}</span>
            </div>
          )}

          <form onSubmit={handleRestoreTest} className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-neutral-300 mb-1">
                Backup File Path to Restore
              </label>
              <input
                type="text"
                value={restoreFile}
                onChange={(e) => setRestoreFile(e.target.value)}
                className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white font-mono focus:border-white focus:outline-none"
                required
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-neutral-300 mb-1">
                Backup Passphrase
              </label>
              <input
                type="password"
                value={restorePassphrase}
                onChange={(e) => setRestorePassphrase(e.target.value)}
                className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none font-mono"
                placeholder="••••••••••••••••"
                required
              />
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={restoring}
                className="w-full py-2.5 bg-[#181818] border border-[#333333] hover:bg-[#222222] disabled:opacity-50 text-white text-xs font-semibold rounded-lg shadow-sm transition"
              >
                {restoring ? 'Verifying & Unpacking...' : 'Verify & Restore Backup Archive'}
              </button>
            </div>
          </form>

          <div className="p-4 rounded-lg bg-[#0d0d0d] border border-[#222222] text-[11px] text-neutral-400 space-y-1">
            <span className="font-semibold text-neutral-200 block">Security Invariant:</span>
            <span>
              Restoration validates Argon2id key derivation, AES-GCM MAC authenticity tag, and SHA-256 payload integrity before overwriting state.
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
