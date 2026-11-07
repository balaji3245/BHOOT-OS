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
    <header className="h-16 border-b border-neutral-800 bg-black/90 backdrop-blur px-8 flex items-center justify-between">
      {/* Context Badges */}
      <div className="flex items-center space-x-3">
        <div className="flex items-center space-x-2 text-xs bg-[#121212] text-neutral-300 px-3 py-1.5 rounded-md border border-neutral-800">
          <Clock className="w-3.5 h-3.5 text-neutral-400" />
          <span>{today}</span>
        </div>

        <div className="flex items-center space-x-1.5 text-xs bg-[#121212] text-neutral-300 px-3 py-1.5 rounded-md border border-neutral-800">
          <WifiOff className="w-3.5 h-3.5 text-neutral-400" />
          <span className="font-medium">Offline Airgapped</span>
        </div>
      </div>

      {/* Action Controls */}
      <div className="flex items-center space-x-3">
        <div className="flex items-center space-x-1.5 text-xs bg-[#121212] text-neutral-300 px-3 py-1.5 rounded-md border border-neutral-800">
          <ShieldCheck className="w-3.5 h-3.5 text-neutral-400" />
          <span>Audit Chain: Active</span>
        </div>

        <button
          onClick={onOpenQuickTx}
          className="flex items-center space-x-1.5 bg-white hover:bg-neutral-200 text-black text-xs font-semibold px-3.5 py-1.5 rounded-lg shadow transition"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>Record Entry</span>
        </button>
      </div>
    </header>
  );
};
