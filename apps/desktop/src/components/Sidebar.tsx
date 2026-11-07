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
    <aside className="w-64 bg-black border-r border-neutral-800 flex flex-col h-screen select-none">
      {/* Brand Header */}
      <div className="p-5 border-b border-neutral-800 flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <div className="w-8 h-8 rounded-lg bg-white flex items-center justify-center text-black font-extrabold text-lg tracking-wider shadow">
            भ
          </div>
          <div>
            <h1 className="font-extrabold text-base text-white tracking-wider">BHOOT</h1>
            <p className="text-[10px] uppercase tracking-widest text-neutral-400 font-medium">
              Financial OS v1.0
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
              className={`w-full flex items-center space-x-3 px-3 py-2 rounded-lg text-xs font-medium transition-all ${
                isActive
                  ? 'bg-white text-black shadow-sm font-semibold'
                  : 'text-neutral-400 hover:text-white hover:bg-neutral-900 border border-transparent'
              }`}
            >
              <Icon className={`w-4 h-4 ${isActive ? 'text-black' : 'text-neutral-400'}`} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>

      {/* Security & Lock Badge Footer */}
      <div className="p-4 border-t border-neutral-800 bg-[#080808]">
        <div className="flex items-center justify-between mb-3 px-1">
          <div className="flex items-center space-x-2">
            <span className="w-2 h-2 rounded-full bg-neutral-400"></span>
            <span className="text-xs text-neutral-300 font-medium">Encrypted Storage</span>
          </div>
          <span className="text-[10px] bg-neutral-800 text-neutral-300 px-2 py-0.5 rounded border border-neutral-700 font-mono tracking-wider">
            OFFLINE
          </span>
        </div>

        <button
          onClick={onLockSession}
          className="w-full flex items-center justify-center space-x-2 py-2 px-3 rounded-lg bg-[#141414] hover:bg-neutral-800 text-neutral-300 hover:text-white text-xs font-semibold border border-neutral-800 transition"
        >
          <Lock className="w-3.5 h-3.5" />
          <span>Lock Financial OS</span>
        </button>
      </div>
    </aside>
  );
};
