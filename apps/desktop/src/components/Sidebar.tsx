import React from 'react';
import {
  LayoutDashboard,
  ArrowLeftRight,
  Wallet,
  PiggyBank,
  TrendingUp,
  FileText,
  Shield,
  FileCheck2,
  HardDriveDownload,
  Settings,
  Lock,
  Layers,
} from 'lucide-react';

export type TabId =
  | 'dashboard'
  | 'transactions'
  | 'accounts'
  | 'budgets'
  | 'assets_liabilities'
  | 'trading'
  | 'reports'
  | 'vault'
  | 'backup'
  | 'security'
  | 'settings';

interface SidebarProps {
  activeTab: TabId;
  setActiveTab: (tab: TabId) => void;
  onLockSession: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ activeTab, setActiveTab, onLockSession }) => {
  const menuItems = [
    { id: 'dashboard' as TabId, label: 'Dashboard', icon: LayoutDashboard },
    { id: 'transactions' as TabId, label: 'Transactions', icon: ArrowLeftRight },
    { id: 'accounts' as TabId, label: 'Accounts & Ledger', icon: Wallet },
    { id: 'budgets' as TabId, label: 'Budgets & Limits', icon: PiggyBank },
    { id: 'assets_liabilities' as TabId, label: 'Assets & Liabilities', icon: Layers },
    { id: 'trading' as TabId, label: 'Trading Journal', icon: TrendingUp },
    { id: 'reports' as TabId, label: 'Financial Reports', icon: FileText },
    { id: 'vault' as TabId, label: 'Document Vault', icon: FileCheck2 },
    { id: 'backup' as TabId, label: 'Backup & Recovery', icon: HardDriveDownload },
    { id: 'security' as TabId, label: 'Security Center', icon: Shield },
    { id: 'settings' as TabId, label: 'Settings', icon: Settings },
  ];

  return (
    <aside className="w-64 bg-[#0d121f] border-r border-slate-800/80 flex flex-col h-screen select-none">
      {/* Brand Header */}
      <div className="p-6 border-b border-slate-800/80 flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <div className="w-9 h-9 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 font-bold text-xl tracking-wider shadow-inner">
            भ
          </div>
          <div>
            <h1 className="font-extrabold text-lg text-white tracking-wide">BHOOT</h1>
            <p className="text-[10px] uppercase tracking-widest text-emerald-400 font-semibold">
              Offline OS v1.0
            </p>
          </div>
        </div>
      </div>

      {/* Navigation Links */}
      <nav className="flex-1 p-3 space-y-1 overflow-y-auto">
        {menuItems.map((item) => {
          const Icon = item.icon;
          const isActive = activeTab === item.id;
          return (
            <button
              key={item.id}
              onClick={() => setActiveTab(item.id)}
              className={`w-full flex items-center space-x-3 px-3.5 py-2.5 rounded-lg text-sm font-medium transition-all ${
                isActive
                  ? 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 shadow-sm'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50 border border-transparent'
              }`}
            >
              <Icon className={`w-4 h-4 ${isActive ? 'text-emerald-400' : 'text-slate-400'}`} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>

      {/* Security & Lock Badge Footer */}
      <div className="p-4 border-t border-slate-800/80 bg-slate-900/40">
        <div className="flex items-center justify-between mb-3 px-1">
          <div className="flex items-center space-x-2">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span className="text-xs text-slate-300 font-medium">LUKS2 + SQLCipher</span>
          </div>
          <span className="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded border border-emerald-500/30 font-mono">
            AIRGAPPED
          </span>
        </div>
        <button
          onClick={onLockSession}
          className="w-full flex items-center justify-center space-x-2 py-2 px-3 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border border-rose-500/30 text-xs font-semibold transition"
        >
          <Lock className="w-3.5 h-3.5" />
          <span>Lock Session</span>
        </button>
      </div>
    </aside>
  );
};
