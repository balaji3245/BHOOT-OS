import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { BalanceSheet } from '../types';
import { formatINR } from '../utils/format';
import { ShieldCheck, ArrowUpRight, ArrowDownRight } from 'lucide-react';

export const AssetsLiabilitiesView: React.FC = () => {
  const [balanceSheet, setBalanceSheet] = useState<BalanceSheet | null>(null);

  useEffect(() => {
    async function load() {
      try {
        const bs = await api.getBalanceSheet();
        setBalanceSheet(bs);
      } catch (err) {
        console.error(err);
      }
    }
    load();
  }, []);

  const totalAssets = balanceSheet?.total_assets_paisa ?? 0;
  const totalLiabilities = balanceSheet?.total_liabilities_paisa ?? 0;
  const netWorth = balanceSheet?.net_worth_paisa ?? 0;

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Assets & Liabilities</h1>
          <p className="text-xs text-slate-400">
            Realized balance sheet portfolio valuation and claim settlement equilibrium
          </p>
        </div>
        <div className="flex items-center space-x-2 text-xs bg-emerald-500/10 text-emerald-400 px-3 py-1.5 rounded-lg border border-emerald-500/30">
          <ShieldCheck className="w-4 h-4" />
          <span>Accounting Equation: Assets = Liabilities + Equity</span>
        </div>
      </div>

      {/* Summary KPI Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
        <div className="p-6 rounded-xl bg-gradient-to-br from-[#131c2e] to-[#0d1424] border border-slate-800 shadow-sm">
          <span className="text-xs uppercase tracking-wider text-slate-400 block mb-1 font-semibold">
            True Net Worth
          </span>
          <span className="text-3xl font-extrabold text-white tracking-tight">
            {formatINR(netWorth)}
          </span>
          <span className="text-xs text-emerald-400 block mt-2">
            Assets ({formatINR(totalAssets)}) - Liabilities ({formatINR(totalLiabilities)})
          </span>
        </div>

        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 shadow-sm">
          <div className="flex items-center justify-between text-slate-400 mb-1">
            <span className="text-xs uppercase tracking-wider font-semibold">Gross Assets</span>
            <ArrowUpRight className="w-4 h-4 text-emerald-400" />
          </div>
          <span className="text-2xl font-bold text-white tracking-tight">
            {formatINR(totalAssets)}
          </span>
          <span className="text-xs text-slate-400 block mt-2">
            {balanceSheet?.assets.length || 0} active asset holdings
          </span>
        </div>

        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 shadow-sm">
          <div className="flex items-center justify-between text-slate-400 mb-1">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Liabilities</span>
            <ArrowDownRight className="w-4 h-4 text-rose-400" />
          </div>
          <span className="text-2xl font-bold text-white tracking-tight">
            {formatINR(totalLiabilities)}
          </span>
          <span className="text-xs text-slate-400 block mt-2">
            {balanceSheet?.liabilities.length || 0} debt claims
          </span>
        </div>
      </div>

      {/* Two Column Side-by-Side Breakdown */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Assets Column */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 space-y-4">
          <div className="flex items-center justify-between border-b border-slate-800 pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-2.5 h-2.5 rounded-full bg-emerald-400"></span>
              <h3 className="text-base font-bold text-white">Assets (Debit Normal)</h3>
            </div>
            <span className="font-mono text-xs font-bold text-emerald-400">
              {formatINR(totalAssets)}
            </span>
          </div>

          <div className="space-y-3">
            {balanceSheet?.assets.map((a) => (
              <div
                key={a.account_id}
                className="p-3.5 rounded-lg bg-slate-900/60 border border-slate-800 flex items-center justify-between"
              >
                <div>
                  <h4 className="text-sm font-semibold text-white">{a.name}</h4>
                  <span className="text-[10px] text-slate-400 font-mono">{a.subtype}</span>
                </div>
                <span className="text-sm font-bold font-mono text-white">
                  {formatINR(a.balance_paisa)}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Liabilities Column */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 space-y-4">
          <div className="flex items-center justify-between border-b border-slate-800 pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-2.5 h-2.5 rounded-full bg-rose-400"></span>
              <h3 className="text-base font-bold text-white">Liabilities (Credit Normal)</h3>
            </div>
            <span className="font-mono text-xs font-bold text-rose-400">
              {formatINR(totalLiabilities)}
            </span>
          </div>

          <div className="space-y-3">
            {balanceSheet?.liabilities.map((l) => (
              <div
                key={l.account_id}
                className="p-3.5 rounded-lg bg-slate-900/60 border border-slate-800 flex items-center justify-between"
              >
                <div>
                  <h4 className="text-sm font-semibold text-white">{l.name}</h4>
                  <span className="text-[10px] text-slate-400 font-mono">{l.subtype}</span>
                </div>
                <span className="text-sm font-bold font-mono text-rose-400">
                  {formatINR(l.balance_paisa)}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
