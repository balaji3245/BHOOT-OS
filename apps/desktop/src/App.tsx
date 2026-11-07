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
import { Lock, ShieldAlert } from 'lucide-react';
import { api } from './api/client';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabId>('dashboard');
  const [txModalOpen, setTxModalOpen] = useState(false);
  const [isLocked, setIsLocked] = useState(true); // Locked by default for zero-trust security
  const [pinInput, setPinInput] = useState('');
  const [lockError, setLockError] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);

  const lastActivityRef = useRef<number>(Date.now());

  // Check initial security status & token validity
  useEffect(() => {
    let mounted = true;
    const checkSecurity = async () => {
      try {
        const token = sessionStorage.getItem('bhoot_session_token');
        const status = await api.getSecurityStatus();
        if (mounted) {
          if (status.session_locked || !token) {
            setIsLocked(true);
          } else {
            setIsLocked(false);
          }
        }
      } catch {
        if (mounted) setIsLocked(true);
      }
    };
    checkSecurity();

    // Activity tracking for auto-lock
    const recordActivity = () => {
      lastActivityRef.current = Date.now();
    };

    window.addEventListener('mousemove', recordActivity, { passive: true });
    window.addEventListener('keydown', recordActivity, { passive: true });
    window.addEventListener('click', recordActivity, { passive: true });

    // Auto-lock interval (checks every 30s; locks if inactive > 15m)
    const interval = setInterval(async () => {
      const inactiveMinutes = (Date.now() - lastActivityRef.current) / 60000;
      if (inactiveMinutes >= 15 && !isLocked) {
        await api.lockSession();
        if (mounted) setIsLocked(true);
      }
    }, 30000);

    return () => {
      mounted = false;
      window.removeEventListener('mousemove', recordActivity);
      window.removeEventListener('keydown', recordActivity);
      window.removeEventListener('click', recordActivity);
      clearInterval(interval);
    };
  }, [isLocked]);

  const handleLock = async () => {
    await api.lockSession();
    setIsLocked(true);
    setPinInput('');
    setLockError(false);
  };

  const handleUnlock = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pinInput) return;
    setIsAuthenticating(true);
    setLockError(false);

    try {
      const success = await api.unlockSession(pinInput);
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
        <Navbar onOpenQuickTx={() => setTxModalOpen(true)} />

        <main className="flex-1 overflow-y-auto">
          {activeTab === 'dashboard' && (
            <DashboardView
              onNavigate={setActiveTab}
              onOpenTxModal={() => setTxModalOpen(true)}
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
