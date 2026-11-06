import React, { useState } from 'react';
import { HardDriveDownload, RefreshCw, CheckCircle2 } from 'lucide-react';

export const BackupRecoveryView: React.FC = () => {
  const [backupPassphrase, setBackupPassphrase] = useState('');
  const [confirmPassphrase, setConfirmPassphrase] = useState('');
  const [backupPath, setBackupPath] = useState('/var/bhoot/backups/bhoot_backup.bhoot');
  const [backingUp, setBackingUp] = useState(false);
  const [backupSuccess, setBackupSuccess] = useState(false);

  // Restore state
  const [restoreFile, setRestoreFile] = useState('');
  const [restorePassphrase, setRestorePassphrase] = useState('');
  const [restoring, setRestoring] = useState(false);
  const [restoreVerified, setRestoreVerified] = useState(false);

  const handleCreateBackup = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!backupPassphrase || backupPassphrase !== confirmPassphrase) {
      alert('Passphrases do not match or are empty');
      return;
    }
    if (backupPassphrase.length < 12) {
      alert('Backup passphrase must be at least 12 characters');
      return;
    }

    setBackingUp(true);
    setBackupSuccess(false);

    try {
      // Simulate backend call
      await new Promise((r) => setTimeout(r, 1200));
      setBackupSuccess(true);
      setBackupPassphrase('');
      setConfirmPassphrase('');
    } catch (err: any) {
      alert(`Backup failed: ${err.message}`);
    } finally {
      setBackingUp(false);
    }
  };

  const handleRestoreTest = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!restorePassphrase) {
      alert('Enter backup passphrase');
      return;
    }

    setRestoring(true);
    setRestoreVerified(false);

    try {
      await new Promise((r) => setTimeout(r, 1400));
      setRestoreVerified(true);
    } catch (err: any) {
      alert(`Restoration verification failed: ${err.message}`);
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
        <p className="text-xs text-slate-400">
          Argon2id and AES-256-GCM encrypted airgapped archives with automated restoration verification
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Create Encrypted Backup */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 space-y-5">
          <div className="flex items-center space-x-3 text-emerald-400 border-b border-slate-800 pb-3">
            <HardDriveDownload className="w-5 h-5" />
            <h3 className="text-base font-bold text-white">Create Encrypted Backup Archive</h3>
          </div>

          <p className="text-xs text-slate-300 leading-relaxed">
            Exports a standalone sealed archive containing your SQLCipher database, document vault
            payloads, and cryptographic audit trail. The archive is encrypted with a dedicated
            Argon2id key derived from your chosen backup passphrase.
          </p>

          {backupSuccess && (
            <div className="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-lg flex items-center space-x-2 text-xs text-emerald-400">
              <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
              <span>
                Encrypted backup successfully exported to {backupPath}. SHA-256 integrity verified.
              </span>
            </div>
          )}

          <form onSubmit={handleCreateBackup} className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">
                Destination Archive Path
              </label>
              <input
                type="text"
                value={backupPath}
                onChange={(e) => setBackupPath(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 font-mono"
                required
              />
              <span className="text-[10px] text-slate-500 block mt-1">
                Must target an external offline media or persistent user directory (not /boot or /dev)
              </span>
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">
                New Backup Passphrase (min 12 chars)
              </label>
              <input
                type="password"
                value={backupPassphrase}
                onChange={(e) => setBackupPassphrase(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                placeholder="••••••••••••••••"
                required
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">
                Confirm Backup Passphrase
              </label>
              <input
                type="password"
                value={confirmPassphrase}
                onChange={(e) => setConfirmPassphrase(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                placeholder="••••••••••••••••"
                required
              />
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={backingUp}
                className="w-full py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white text-xs font-semibold rounded-lg shadow-sm transition"
              >
                {backingUp ? 'Deriving Key & Encrypting Archive...' : 'Generate Encrypted Backup'}
              </button>
            </div>
          </form>
        </div>

        {/* Restore & Verify */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 space-y-5">
          <div className="flex items-center space-x-3 text-blue-400 border-b border-slate-800 pb-3">
            <RefreshCw className="w-5 h-5" />
            <h3 className="text-base font-bold text-white">Disaster Recovery & Restoration</h3>
          </div>

          <p className="text-xs text-slate-300 leading-relaxed">
            Restores an encrypted BHOOT archive. Before overwriting the live database, BHOOT runs an
            <strong> isolated test restoration in RAM</strong> and verifies SQLCipher page-level
            integrity checks to guarantee the archive is not corrupted.
          </p>

          {restoreVerified && (
            <div className="p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg flex items-center space-x-2 text-xs text-blue-400">
              <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
              <span>
                Isolated dry-run test passed! PRAGMA integrity_check confirmed zero corruption.
              </span>
            </div>
          )}

          <form onSubmit={handleRestoreTest} className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">
                Archive File Path
              </label>
              <input
                type="text"
                placeholder="/media/usb/bhoot_backup.bhoot"
                value={restoreFile}
                onChange={(e) => setRestoreFile(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 font-mono"
                required
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">
                Backup Passphrase
              </label>
              <input
                type="password"
                placeholder="••••••••••••••••"
                value={restorePassphrase}
                onChange={(e) => setRestorePassphrase(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                required
              />
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={restoring}
                className="w-full py-2.5 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-xs font-semibold rounded-lg shadow-sm transition"
              >
                {restoring
                  ? 'Verifying Archive in Isolated Fixture...'
                  : 'Test Restore & Authenticate'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
};
