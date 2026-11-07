import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Moon, DollarSign, Info, Shield, Clock, CheckCircle2 } from 'lucide-react';

export const SettingsView: React.FC = () => {
  const [currency, setCurrency] = useState('INR');
  const [theme, setTheme] = useState('dark');
  const [autoLockMinutes, setAutoLockMinutes] = useState(15);
  const [financialYearStart, setFinancialYearStart] = useState('04-01');
  const [savedSuccess, setSavedSuccess] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      try {
        const s = await api.getSettings();
        setCurrency(s.currency || 'INR');
        setTheme(s.theme || 'dark');
        setAutoLockMinutes(s.auto_lock_minutes || 15);
        setFinancialYearStart(s.financial_year_start || '04-01');
      } catch (err) {
        console.error('Failed to load settings:', err);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  const handleSaveSettings = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await api.updateSettings({
        currency,
        theme,
        auto_lock_minutes: Number(autoLockMinutes),
        offline_mode: true,
        financial_year_start: financialYearStart,
      });
      setSavedSuccess(true);
      setTimeout(() => setSavedSuccess(false), 3000);
    } catch (err: any) {
      alert(`Failed to save settings: ${err.message}`);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Loading Preferences...
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-8 max-w-4xl mx-auto">
      <div>
        <h1 className="text-2xl font-extrabold text-white tracking-tight">System Settings</h1>
        <p className="text-xs text-neutral-400">
          Personalization and local offline configuration parameters
        </p>
      </div>

      {savedSuccess && (
        <div className="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-lg flex items-center space-x-2 text-xs text-emerald-400">
          <CheckCircle2 className="w-4 h-4" />
          <span>Settings saved to persistent offline configuration!</span>
        </div>
      )}

      <form onSubmit={handleSaveSettings} className="space-y-6">
        <div className="bg-[#111111] rounded-xl border border-[#222222] divide-y divide-[#222222]">
          {/* Currency Setting */}
          <div className="p-6 flex items-center justify-between">
            <div className="space-y-1">
              <h4 className="text-sm font-bold text-white flex items-center space-x-2">
                <DollarSign className="w-4 h-4 text-white" />
                <span>Base Ledger Currency</span>
              </h4>
              <p className="text-xs text-neutral-400">
                Primary currency unit utilized for double-entry ledger equilibrium calculations
              </p>
            </div>
            <select
              value={currency}
              onChange={(e) => setCurrency(e.target.value)}
              className="bg-[#181818] border border-[#333333] text-xs rounded-lg px-3 py-2 text-white focus:border-white focus:outline-none font-mono"
            >
              <option value="INR">INR — Indian Rupee (₹)</option>
            </select>
          </div>

          {/* Inactivity Auto-Lock */}
          <div className="p-6 flex items-center justify-between">
            <div className="space-y-1">
              <h4 className="text-sm font-bold text-white flex items-center space-x-2">
                <Clock className="w-4 h-4 text-white" />
                <span>Inactivity Session Lock</span>
              </h4>
              <p className="text-xs text-neutral-400">
                Automatically seals volatile application memory if no input is detected
              </p>
            </div>
            <select
              value={autoLockMinutes}
              onChange={(e) => setAutoLockMinutes(Number(e.target.value))}
              className="bg-[#181818] border border-[#333333] text-xs rounded-lg px-3 py-2 text-white focus:border-white focus:outline-none font-mono"
            >
              <option value={5}>5 minutes</option>
              <option value={10}>10 minutes</option>
              <option value={15}>15 minutes (Standard)</option>
              <option value={30}>30 minutes</option>
              <option value={60}>60 minutes</option>
            </select>
          </div>

          {/* Financial Year Start */}
          <div className="p-6 flex items-center justify-between">
            <div className="space-y-1">
              <h4 className="text-sm font-bold text-white flex items-center space-x-2">
                <Shield className="w-4 h-4 text-white" />
                <span>Financial Year Convention</span>
              </h4>
              <p className="text-xs text-neutral-400">
                Statutory fiscal cycle starting date for P&L calculations
              </p>
            </div>
            <select
              value={financialYearStart}
              onChange={(e) => setFinancialYearStart(e.target.value)}
              className="bg-[#181818] border border-[#333333] text-xs rounded-lg px-3 py-2 text-white focus:border-white focus:outline-none font-mono"
            >
              <option value="04-01">April 1st (Indian Standard Fiscal Year)</option>
              <option value="01-01">January 1st (Calendar Year)</option>
            </select>
          </div>

          {/* Theme Setting */}
          <div className="p-6 flex items-center justify-between">
            <div className="space-y-1">
              <h4 className="text-sm font-bold text-white flex items-center space-x-2">
                <Moon className="w-4 h-4 text-white" />
                <span>Visual Theme</span>
              </h4>
              <p className="text-xs text-neutral-400">
                Premium high-contrast monochrome black & white architecture
              </p>
            </div>
            <div className="text-xs font-mono px-3 py-1.5 rounded-lg bg-[#181818] text-white border border-[#333333]">
              Monochrome Dark
            </div>
          </div>

          {/* About BHOOT OS */}
          <div className="p-6 space-y-3">
            <h4 className="text-sm font-bold text-white flex items-center space-x-2">
              <Info className="w-4 h-4 text-white" />
              <span>About BHOOT Financial Operating System</span>
            </h4>
            <div className="text-xs text-neutral-400 space-y-1.5 leading-relaxed font-mono">
              <p>
                <strong>BHOOT</strong>: Autonomous, Offline-First, Private Financial OS
              </p>
              <p>Version: <span className="text-white">1.0.0 Production Release</span></p>
              <p>Storage Engine: <span className="text-white">SQLCipher 4.19 + Argon2id LUKS2</span></p>
              <p>Network Boundary: <span className="text-white">Strict Airgap / 127.0.0.1 Isolated</span></p>
            </div>
          </div>
        </div>

        <div className="flex justify-end">
          <button
            type="submit"
            className="px-5 py-2.5 bg-white hover:bg-neutral-200 text-black text-xs font-semibold rounded-lg shadow-sm transition"
          >
            Save Settings
          </button>
        </div>
      </form>
    </div>
  );
};
