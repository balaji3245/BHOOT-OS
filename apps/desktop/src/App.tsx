import React, { useState, useEffect, useRef } from 'react';
import { Sidebar, TabId } from './components/Sidebar';
import { Navbar } from './components/Navbar';
import { DashboardView } from './views/DashboardView';
import { TransactionsView } from './views/TransactionsView';
import { AccountsView } from './views/AccountsView';
import { BudgetsView } from './views/BudgetsView';
import { AssetsLiabilitiesView } from './views/AssetsLiabilitiesView';
import { TradingJournalView } from './views/TradingJournalView';
import { ReportsView } from './views/ReportsView';
import { VaultView } from './views/VaultView';
import { BackupRecoveryView } from './views/BackupRecoveryView';
import { SecurityCenterView } from './views/SecurityCenterView';
import { SettingsView } from './views/SettingsView';
import { Lock, ShieldAlert, KeyRound } from 'lucide-react';
import { api } from './api/client';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabId>('dashboard');
  const [txModalOpen, setTxModalOpen] = useState(false);
  const [isLocked, setIsLocked] = useState(true); // Locked by default for zero-trust security
  const [isEnrolled, setIsEnrolled] = useState<boolean | null>(null);
  const [pinInput, setPinInput] = useState('');
  const [lockError, setLockError] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);

  // First-owner enrollment state
  const [enrollPin, setEnrollPin] = useState('');
  const [enrollConfirmPin, setEnrollConfirmPin] = useState('');
  const [enrollError, setEnrollError] = useState<string | null>(null);
  const [isEnrolling, setIsEnrolling] = useState(false);
  const [autoLockMinutes, setAutoLockMinutes] = useState(15);

  const lastActivityRef = useRef<number>(Date.now());
  const pinInputRef = useRef<string>('');
  pinInputRef.current = pinInput;
  const isEnrolledRef = useRef<boolean | null>(isEnrolled);
  isEnrolledRef.current = isEnrolled;
  const autoLockMinutesRef = useRef<number>(15);
  autoLockMinutesRef.current = autoLockMinutes;

  // Check initial security status & token validity
  useEffect(() => {
    let mounted = true;
    const checkSecurity = async () => {
      try {
        const token = sessionStorage.getItem('bhoot_session_token');
        const status = await api.getSecurityStatus();
        if (mounted) {
          setIsEnrolled(Boolean(status.auth_enrolled));
          if (!status.auth_enrolled) {
            setIsLocked(true);
          } else if (status.session_locked || !token) {
            setIsLocked(true);
          } else {
            setIsLocked(false);
            // Load user preferences for session timeout
            try {
              const s = await api.getSettings();
              if (s && s.auto_lock_minutes) {
                setAutoLockMinutes(s.auto_lock_minutes);
                autoLockMinutesRef.current = s.auto_lock_minutes;
              }
            } catch {
              // ignore
            }
          }
        }
      } catch {
        if (mounted) {
          setIsEnrolled(false);
          setIsLocked(true);
        }
      }
    };
    checkSecurity();

    // Fast sync interval while unenrolled to transition as soon as setup completes
    const syncInterval = setInterval(async () => {
      if (!isEnrolledRef.current) {
        try {
          const status = await api.getSecurityStatus();
          if (mounted && status.auth_enrolled) {
            setIsEnrolled(true);
            setIsLocked(true);
          }
        } catch {
          // ignore
        }
      }
    }, 1500);

    // Activity tracking & global navigation shortcuts
    const recordActivity = () => {
      lastActivityRef.current = Date.now();
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      recordActivity();
      const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
      if (tag === 'input' || tag === 'textarea' || tag === 'select') return;

      if (isEnrolledRef.current === false) {
        return;
      }

      if (isLocked) {
        if (e.key === 'Enter') {
          doUnlock();
        } else if (/^[0-9]$/.test(e.key)) {
          setPinInput((prev) => {
            const next = prev + e.key;
            pinInputRef.current = next;
            return next;
          });
        } else if (e.key === 'Backspace') {
          setPinInput((prev) => {
            const next = prev.slice(0, -1);
            pinInputRef.current = next;
            return next;
          });
        }
        return;
      }
      if (e.key === '2' || (e.altKey && e.key === '2') || e.key === 't') {
        setActiveTab('transactions');
      } else if (e.key === '1' || (e.altKey && e.key === '1') || e.key === 'd') {
        setActiveTab('dashboard');
      } else if (e.key === 'l' || (e.altKey && e.key === 'l')) {
        handleLock();
      }
    };

    window.addEventListener('mousemove', recordActivity, { passive: true });
    window.addEventListener('keydown', handleKeyDown, { passive: true });
    window.addEventListener('click', recordActivity, { passive: true });

    // Auto-lock interval (checks every 30s; locks if inactive > configured autoLockMinutes)
    const interval = setInterval(async () => {
      const inactiveMinutes = (Date.now() - lastActivityRef.current) / 60000;
      const timeoutLimit = autoLockMinutesRef.current || 15;
      if (inactiveMinutes >= timeoutLimit && !isLocked) {
        await api.lockSession();
        if (mounted) setIsLocked(true);
      }
    }, 30000);

    return () => {
      mounted = false;
      window.removeEventListener('mousemove', recordActivity);
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('click', recordActivity);
      clearInterval(syncInterval);
      clearInterval(interval);
    };
  }, [isLocked]);

  const handleLock = async () => {
    await api.lockSession();
    setIsLocked(true);
    setPinInput('');
    setLockError(false);
  };

  const doUnlock = async (overridePin?: string) => {
    const pin = (overridePin || pinInput || pinInputRef.current).trim();
    if (!pin) return;
    setIsAuthenticating(true);
    setLockError(false);

    try {
      const success = await api.unlockSession(pin);
      if (success) {
        setIsLocked(false);
        setPinInput('');
        setLockError(false);
        lastActivityRef.current = Date.now();
      } else {
        setLockError(true);
      }
    } catch {
      setLockError(true);
    } finally {
      setIsAuthenticating(false);
    }
  };

  const handleEnroll = async (e: React.FormEvent) => {
    e.preventDefault();
    setEnrollError(null);

    if (enrollPin !== enrollConfirmPin) {
      setEnrollError('PIN confirmation does not match.');
      return;
    }
    if (enrollPin.length < 6) {
      setEnrollError('PIN must be at least 6 characters in length.');
      return;
    }

    setIsEnrolling(true);
    try {
      const success = await api.enrollSession(enrollPin, enrollConfirmPin);
      if (success) {
        setIsEnrolled(true);
        setIsLocked(false);
        setEnrollPin('');
        setEnrollConfirmPin('');
        lastActivityRef.current = Date.now();
      } else {
        setEnrollError('Enrollment failed. Please check PIN strength.');
      }
    } catch (err: any) {
      setEnrollError(err.message || 'Enrollment failed.');
    } finally {
      setIsEnrolling(false);
    }
  };

  const handleUnlock = async (e: React.FormEvent) => {
    e.preventDefault();
    await doUnlock(pinInput);
  };

  // 1. Mandatory First-Owner PIN Enrollment Screen
  if (isEnrolled === false) {
    return (
      <div className="fixed inset-0 z-50 bg-[#000000] flex items-center justify-center p-4">
        <div className="max-w-md w-full bg-[#0d0d0d] border border-neutral-800 rounded-2xl p-8 space-y-6 shadow-2xl text-center">
          <div className="w-16 h-16 rounded-full bg-neutral-900 border border-neutral-700 flex items-center justify-center text-white mx-auto shadow-inner">
            <KeyRound className="w-8 h-8" />
          </div>

          <div>
            <h2 className="text-xl font-bold tracking-tight text-white uppercase">BHOOT OS Setup</h2>
            <p className="text-xs text-neutral-400 mt-1">
              First-time setup required. Establish your master owner PIN to protect offline financial records.
            </p>
          </div>

          <form onSubmit={handleEnroll} className="space-y-4 text-left">
            <div>
              <label className="block text-[11px] font-medium text-neutral-400 uppercase tracking-wider mb-1 text-center">
                Create Master PIN
              </label>
              <input
                type="password"
                placeholder="Enter Master PIN (min 6 chars)"
                value={enrollPin}
                onChange={(e) => setEnrollPin(e.target.value)}
                className="w-full bg-[#141414] border border-neutral-800 focus:border-white text-center text-sm rounded-xl py-3 px-4 text-white placeholder-neutral-600 focus:outline-none tracking-widest transition"
                autoFocus
                required
              />
            </div>

            <div>
              <label className="block text-[11px] font-medium text-neutral-400 uppercase tracking-wider mb-1 text-center">
                Confirm Master PIN
              </label>
              <input
                type="password"
                placeholder="Confirm Master PIN"
                value={enrollConfirmPin}
                onChange={(e) => setEnrollConfirmPin(e.target.value)}
                className="w-full bg-[#141414] border border-neutral-800 focus:border-white text-center text-sm rounded-xl py-3 px-4 text-white placeholder-neutral-600 focus:outline-none tracking-widest transition"
                required
              />
            </div>

            <p className="text-[10px] text-neutral-500 text-center">
              Policy: Min 6 chars. No repetitive (111111) or sequential (123456) patterns.
            </p>

            {enrollError && (
              <div className="flex items-center justify-center space-x-1.5 text-xs text-red-400 text-center">
                <ShieldAlert className="w-4 h-4 flex-shrink-0" />
                <span>{enrollError}</span>
              </div>
            )}

            <button
              type="submit"
              disabled={isEnrolling}
              className="w-full py-3 bg-white hover:bg-neutral-200 disabled:opacity-50 text-black font-semibold text-xs tracking-wider uppercase rounded-xl shadow transition"
            >
              {isEnrolling ? 'Enrolling & Hashing (Argon2id)...' : 'Complete Setup & Unlock'}
            </button>
          </form>

          <div className="pt-2 border-t border-neutral-900">
            <span className="text-[10px] font-mono text-neutral-500 uppercase tracking-widest">
              Argon2id Key Derivation • Persistent SQLCipher Storage
            </span>
          </div>
        </div>
      </div>
    );
  }

  // 2. Standard Session Lock Screen (When Enrolled)
  if (isLocked) {
    return (
      <div className="fixed inset-0 z-50 bg-[#000000] flex items-center justify-center p-4">
        <div className="max-w-md w-full bg-[#0d0d0d] border border-neutral-800 rounded-2xl p-8 space-y-6 shadow-2xl text-center">
          <div className="w-16 h-16 rounded-full bg-neutral-900 border border-neutral-700 flex items-center justify-center text-white mx-auto shadow-inner">
            <Lock className="w-8 h-8" />
          </div>

          <div>
            <h2 className="text-xl font-bold tracking-tight text-white uppercase">BHOOT OS Locked</h2>
            <p className="text-xs text-neutral-400 mt-1">
              Argon2id protected memory state active. Enter master PIN to access financial records.
            </p>
          </div>

          <form onSubmit={handleUnlock} className="space-y-4">
            <div className="relative">
              <input
                type="password"
                placeholder="Enter Master PIN"
                value={pinInput}
                onChange={(e) => setPinInput(e.target.value)}
                className={`w-full bg-[#141414] border text-center text-sm rounded-xl py-3 px-4 text-white placeholder-neutral-600 focus:outline-none tracking-widest transition ${
                  lockError ? 'border-red-500 focus:border-red-500' : 'border-neutral-800 focus:border-white'
                }`}
                autoFocus
                required
              />
            </div>

            {lockError && (
              <div className="flex items-center justify-center space-x-1.5 text-xs text-red-400">
                <ShieldAlert className="w-4 h-4 flex-shrink-0" />
                <span>Incorrect credentials. Access denied.</span>
              </div>
            )}

            <button
              type="submit"
              disabled={isAuthenticating}
              className="w-full py-3 bg-white hover:bg-neutral-200 disabled:opacity-50 text-black font-semibold text-xs tracking-wider uppercase rounded-xl shadow transition"
            >
              {isAuthenticating ? 'Authenticating...' : 'Unlock System'}
            </button>
          </form>

          <div className="pt-2 border-t border-neutral-900">
            <span className="text-[10px] font-mono text-neutral-500 uppercase tracking-widest">
              Airgapped Localhost • Zero External Telemetry
            </span>
          </div>
        </div>
      </div>
    );
  }

  const handleOpenTxModal = () => {
    setActiveTab('transactions');
    setTxModalOpen(true);
  };

  return (
    <div className="flex h-screen bg-[#050505] text-neutral-100 overflow-hidden font-sans">
      {/* Navigation Sidebar */}
      <Sidebar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onLockSession={handleLock}
      />

      {/* Main View Area */}
      <div className="flex-1 flex flex-col h-screen overflow-hidden bg-[#050505]">
        <Navbar onOpenQuickTx={handleOpenTxModal} />

        <main className="flex-1 overflow-y-auto">
          {activeTab === 'dashboard' && (
            <DashboardView
              onNavigate={setActiveTab}
              onOpenTxModal={handleOpenTxModal}
            />
          )}
          {activeTab === 'transactions' && (
            <TransactionsView
              txModalOpen={txModalOpen}
              setTxModalOpen={setTxModalOpen}
            />
          )}
          {activeTab === 'accounts' && <AccountsView />}
          {activeTab === 'budgets' && <BudgetsView />}
          {activeTab === 'assets_liabilities' && <AssetsLiabilitiesView />}
          {activeTab === 'trading' && <TradingJournalView />}
          {activeTab === 'reports' && <ReportsView />}
          {activeTab === 'vault' && <VaultView />}
          {activeTab === 'backup' && <BackupRecoveryView />}
          {activeTab === 'security' && <SecurityCenterView onLockSession={handleLock} />}
          {activeTab === 'settings' && <SettingsView />}
        </main>
      </div>
    </div>
  );
};
