import React, { useState } from 'react';
import { Moon, Sun, DollarSign, Info } from 'lucide-react';

export const SettingsView: React.FC = () => {
  const [currency, setCurrency] = useState('INR');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');

  return (
    <div className="p-8 space-y-8 max-w-4xl mx-auto">
      <div>
        <h1 className="text-2xl font-extrabold text-white tracking-tight">System Settings</h1>
        <p className="text-xs text-slate-400">
          Personalization and local offline configuration parameters
        </p>
      </div>

      <div className="bg-[#111726] rounded-xl border border-slate-800 divide-y divide-slate-800/80">
        {/* Currency Setting */}
        <div className="p-6 flex items-center justify-between">
          <div className="space-y-1">
            <h4 className="text-sm font-bold text-white flex items-center space-x-2">
              <DollarSign className="w-4 h-4 text-emerald-400" />
              <span>Default Base Currency</span>
            </h4>
            <p className="text-xs text-slate-400">
              Currency unit utilized for double-entry ledger equilibrium calculations
            </p>
          </div>
          <select
            value={currency}
            onChange={(e) => setCurrency(e.target.value)}
            className="bg-slate-900 border border-slate-700 text-xs rounded-lg px-3 py-2 text-slate-200"
          >
            <option value="INR">INR — Indian Rupee (₹)</option>
          </select>
        </div>

        {/* Theme Setting */}
        <div className="p-6 flex items-center justify-between">
          <div className="space-y-1">
            <h4 className="text-sm font-bold text-white flex items-center space-x-2">
              {theme === 'dark' ? (
                <Moon className="w-4 h-4 text-blue-400" />
              ) : (
                <Sun className="w-4 h-4 text-amber-400" />
              )}
              <span>Interface Theme</span>
            </h4>
            <p className="text-xs text-slate-400">
              High-contrast dark mode optimized for financial analysis and low power consumption
            </p>
          </div>
          <div className="flex bg-slate-900 border border-slate-700 rounded-lg p-1 text-xs">
            <button
              onClick={() => setTheme('dark')}
              className={`px-3 py-1 rounded font-semibold transition ${
                theme === 'dark' ? 'bg-slate-800 text-white shadow-sm' : 'text-slate-400'
              }`}
            >
              Dark
            </button>
            <button
              onClick={() => setTheme('light')}
              className={`px-3 py-1 rounded font-semibold transition ${
                theme === 'light' ? 'bg-slate-800 text-white shadow-sm' : 'text-slate-400'
              }`}
            >
              Light
            </button>
          </div>
        </div>

        {/* About BHOOT OS */}
        <div className="p-6 space-y-3">
          <h4 className="text-sm font-bold text-white flex items-center space-x-2">
            <Info className="w-4 h-4 text-slate-400" />
            <span>About BHOOT Financial Operating System</span>
          </h4>
          <div className="text-xs text-slate-400 space-y-1.5 leading-relaxed">
            <p>
              <strong>BHOOT</strong> is a private, self-contained, offline-first operating system
              engineered for personal financial sovereignty.
            </p>
            <p>
              Version: <span className="text-slate-200 font-mono">1.0.0 (Production Release)</span>
            </p>
            <p>
              Architecture:{' '}
              <span className="text-slate-200 font-mono">
                Debian 12 Live + LUKS2 Argon2id + SQLCipher 4.19 + Rust
              </span>
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};
