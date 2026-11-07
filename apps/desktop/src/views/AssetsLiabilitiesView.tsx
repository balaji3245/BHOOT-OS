import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { BalanceSheet } from '../types';
import { formatINR } from '../utils/format';
import { ShieldCheck, ArrowUpRight, ArrowDownRight } from 'lucide-react';

export const AssetsLiabilitiesView: React.FC = () => {
  const [balanceSheet, setBalanceSheet] = useState<BalanceSheet | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      try {
        const bs = await api.getBalanceSheet();
        setBalanceSheet(bs);
      } catch (err) {
        console.error('Failed to load balance sheet:', err);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  const totalAssets = balanceSheet?.total_assets_paisa ?? 0;
  const totalLiabilities = balanceSheet?.total_liabilities_paisa ?? 0;
  const netWorth = balanceSheet?.net_worth_paisa ?? 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Loading Portfolio Balance Sheet...
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Assets & Liabilities</h1>
          <p className="text-xs text-neutral-400">
            Realized balance sheet portfolio valuation and claim settlement equilibrium
          </p>
        </div>
        <div className="flex items-center space-x-2 text-xs bg-[#111111] text-emerald-400 px-3 py-1.5 rounded-lg border border-[#222222]">
          <ShieldCheck className="w-4 h-4" />
          <span>Accounting Equation: Assets = Liabilities + Equity</span>
        </div>
      </div>

      {/* Summary KPI Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] shadow-sm">
          <span className="text-xs uppercase tracking-wider text-neutral-400 block mb-1 font-semibold">
            True Net Worth
          </span>
          <span className="text-3xl font-extrabold text-white tracking-tight font-mono">
            {formatINR(netWorth)}
          </span>
          <span className="text-xs text-neutral-400 block mt-2 font-mono">
            Assets ({formatINR(totalAssets)}) - Liabilities ({formatINR(totalLiabilities)})
          </span>
        </div>

        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] shadow-sm">
          <div className="flex items-center justify-between text-neutral-400 mb-1">
            <span className="text-xs uppercase tracking-wider font-semibold">Gross Assets</span>
            <ArrowUpRight className="w-4 h-4 text-neutral-400" />
          </div>
          <span className="text-2xl font-bold text-white tracking-tight font-mono">
            {formatINR(totalAssets)}
          </span>
          <span className="text-xs text-neutral-400 block mt-2">
            {balanceSheet?.assets.length || 0} active asset holdings
          </span>
        </div>

        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] shadow-sm">
          <div className="flex items-center justify-between text-neutral-400 mb-1">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Liabilities</span>
            <ArrowDownRight className="w-4 h-4 text-neutral-400" />
          </div>
          <span className="text-2xl font-bold text-white tracking-tight font-mono">
            {formatINR(totalLiabilities)}
          </span>
          <span className="text-xs text-neutral-400 block mt-2">
            {balanceSheet?.liabilities.length || 0} debt claims
          </span>
        </div>
      </div>

      {/* Two Column Side-by-Side Breakdown */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Assets Column */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] space-y-4">
          <div className="flex items-center justify-between border-b border-[#222222] pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-2.5 h-2.5 rounded-full bg-white"></span>
              <h3 className="text-base font-bold text-white">Assets (Debit Normal)</h3>
            </div>
            <span className="font-mono text-xs font-bold text-white">
              {formatINR(totalAssets)}
            </span>
          </div>

          <div className="space-y-3">
            {balanceSheet?.assets.map((a) => (
              <div
                key={a.account_id}
                className="p-3.5 rounded-lg bg-[#181818] border border-[#222222] flex items-center justify-between"
              >
                <div>
                  <h4 className="text-sm font-semibold text-white">{a.name}</h4>
                  <span className="text-[10px] text-neutral-400 font-mono">{a.subtype}</span>
                </div>
                <span className="text-sm font-bold font-mono text-white">
                  {formatINR(a.balance_paisa)}
                </span>
              </div>
            ))}
            {(!balanceSheet?.assets || balanceSheet.assets.length === 0) && (
              <p className="text-xs text-neutral-500 py-4 text-center">No assets recorded</p>
            )}
          </div>
        </div>

        {/* Liabilities Column */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] space-y-4">
          <div className="flex items-center justify-between border-b border-[#222222] pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-2.5 h-2.5 rounded-full bg-neutral-500"></span>
              <h3 className="text-base font-bold text-white">Liabilities (Credit Normal)</h3>
            </div>
            <span className="font-mono text-xs font-bold text-neutral-300">
              {formatINR(totalLiabilities)}
            </span>
          </div>

          <div className="space-y-3">
            {balanceSheet?.liabilities.map((l) => (
              <div
                key={l.account_id}
                className="p-3.5 rounded-lg bg-[#181818] border border-[#222222] flex items-center justify-between"
              >
                <div>
                  <h4 className="text-sm font-semibold text-white">{l.name}</h4>
                  <span className="text-[10px] text-neutral-400 font-mono">{l.subtype}</span>
                </div>
                <span className="text-sm font-bold font-mono text-red-400">
                  {formatINR(l.balance_paisa)}
                </span>
              </div>
            ))}
            {(!balanceSheet?.liabilities || balanceSheet.liabilities.length === 0) && (
              <p className="text-xs text-neutral-500 py-4 text-center">No liabilities or claims</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
