import React from 'react';
import { ShieldCheck, Plus, Clock, WifiOff } from 'lucide-react';

interface NavbarProps {
  onOpenQuickTx: () => void;
}

export const Navbar: React.FC<NavbarProps> = ({ onOpenQuickTx }) => {
  const today = new Date().toLocaleDateString('en-IN', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });

  return (
    <header className="h-16 border-b border-slate-800/80 bg-[#0d121f]/90 backdrop-blur px-8 flex items-center justify-between">
      {/* Search / Title Context */}
      <div className="flex items-center space-x-4">
        <div className="flex items-center space-x-2 text-xs bg-slate-800/60 text-slate-300 px-3 py-1.5 rounded-md border border-slate-700/60">
          <Clock className="w-3.5 h-3.5 text-slate-400" />
          <span>{today}</span>
        </div>

        <div className="flex items-center space-x-1.5 text-xs bg-emerald-500/10 text-emerald-400 px-3 py-1.5 rounded-md border border-emerald-500/20">
          <WifiOff className="w-3.5 h-3.5" />
          <span className="font-medium">100% Offline Airgapped Mode</span>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex items-center space-x-3">
        <div className="flex items-center space-x-1.5 text-xs bg-blue-500/10 text-blue-400 px-3 py-1.5 rounded-md border border-blue-500/20">
          <ShieldCheck className="w-3.5 h-3.5" />
          <span>Audit Chain: Continuous</span>
        </div>

        <button
          onClick={onOpenQuickTx}
          className="flex items-center space-x-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold px-4 py-2 rounded-lg shadow-sm transition"
        >
          <Plus className="w-4 h-4" />
          <span>Post Transaction</span>
        </button>
      </div>
    </header>
  );
};
