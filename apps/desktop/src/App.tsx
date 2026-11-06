import React, { useState } from 'react';
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
import { Lock } from 'lucide-react';
import { api } from './api/client';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabId>('dashboard');
  const [txModalOpen, setTxModalOpen] = useState(false);
  const [isLocked, setIsLocked] = useState(false);
  const [pinInput, setPinInput] = useState('');
  const [lockError, setLockError] = useState(false);

  const handleLock = () => {
    setIsLocked(true);
    setPinInput('');
    setLockError(false);
  };

  const handleUnlock = async (e: React.FormEvent) => {
    e.preventDefault();
    const success = await api.unlockSession(pinInput);
    if (success) {
      setIsLocked(false);
      setPinInput('');
      setLockError(false);
    } else {
      setLockError(true);
    }
  };

  if (isLocked) {
    return (
      <div className="fixed inset-0 z-50 bg-[#0a0d14] flex items-center justify-center p-4">
        <div className="max-w-md w-full bg-[#111726] border border-slate-800 rounded-2xl p-8 space-y-6 shadow-2xl text-center">
          <div className="w-16 h-16 rounded-full bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 mx-auto">
            <Lock className="w-8 h-8" />
          </div>

          <div>
            <h2 className="text-xl font-extrabold text-white">BHOOT Session Locked</h2>
            <p className="text-xs text-slate-400 mt-1">
              Encrypted memory state active. Enter master PIN to resume.
            </p>
          </div>

          <form onSubmit={handleUnlock} className="space-y-4">
            <div className="relative">
              <input
                type="password"
                placeholder="Enter Master PIN / Password"
                value={pinInput}
                onChange={(e) => setPinInput(e.target.value)}
                className={`w-full bg-slate-900 border text-center text-sm rounded-xl py-3 px-4 text-white focus:outline-none tracking-widest ${
                  lockError ? 'border-rose-500' : 'border-slate-700 focus:border-emerald-500'
                }`}
                autoFocus
                required
              />
            </div>

            {lockError && (
              <p className="text-xs text-rose-400 font-medium">
                Incorrect credentials. Enter at least 4 characters.
              </p>
            )}

            <button
              type="submit"
              className="w-full py-3 bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs rounded-xl shadow-md transition"
            >
              Unlock Financial OS
            </button>
          </form>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-[#0a0d14] text-slate-100 overflow-hidden font-sans">
      {/* Navigation Sidebar */}
      <Sidebar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onLockSession={handleLock}
      />

      {/* Main View Area */}
      <div className="flex-1 flex flex-col h-screen overflow-hidden">
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
