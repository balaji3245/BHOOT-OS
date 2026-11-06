import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { BalanceSheet, IncomeStatement, TradingSummary, Transaction } from '../types';
import { formatINR, formatBps, formatDate } from '../utils/format';
import {
  TrendingUp,
  Wallet,
  ArrowUpRight,
  ArrowDownRight,
  Layers,
  ArrowRight,
} from 'lucide-react';

interface DashboardViewProps {
  onNavigate: (tab: any) => void;
  onOpenTxModal: () => void;
}

export const DashboardView: React.FC<DashboardViewProps> = ({ onNavigate, onOpenTxModal }) => {
  const [balanceSheet, setBalanceSheet] = useState<BalanceSheet | null>(null);
  const [incomeStatement, setIncomeStatement] = useState<IncomeStatement | null>(null);
  const [tradingSummary, setTradingSummary] = useState<TradingSummary | null>(null);
  const [recentTxs, setRecentTxs] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadData() {
      try {
        const bs = await api.getBalanceSheet();
        const thirtyDaysAgo = new Date(Date.now() - 30 * 86400000).toISOString();
        const now = new Date().toISOString();
        const pnl = await api.getIncomeStatement(thirtyDaysAgo, now);
        const ts = await api.getTradingSummary();
        const txs = await api.getTransactions(6, 0);

        setBalanceSheet(bs);
        setIncomeStatement(pnl);
        setTradingSummary(ts);
        setRecentTxs(txs);
      } catch (err) {
        console.error('Failed to load dashboard data:', err);
      } finally {
        setLoading(false);
      }
    }
    loadData();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-emerald-400 animate-pulse text-sm">Loading secure ledger...</div>
      </div>
    );
  }

  const netWorth = balanceSheet?.net_worth_paisa ?? 0;
  const totalAssets = balanceSheet?.total_assets_paisa ?? 0;
  const totalLiabilities = balanceSheet?.total_liabilities_paisa ?? 0;
  const monthlyProfit = incomeStatement?.net_profit_paisa ?? 0;
  const winRate = tradingSummary?.win_rate_basis_points ?? 0;
  const tradingNet = tradingSummary?.net_pnl_paisa ?? 0;

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      {/* Hero Financial Summary Banner */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-5">
        {/* Net Worth */}
        <div className="p-6 rounded-xl bg-gradient-to-br from-[#131c2e] to-[#0d1424] border border-slate-800 shadow-sm relative overflow-hidden">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Net Worth</span>
            <Wallet className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="text-3xl font-extrabold text-white tracking-tight">
            {formatINR(netWorth)}
          </div>
          <div className="mt-3 flex items-center space-x-2 text-xs">
            <span className="text-emerald-400 font-medium">Equilibrium Verified</span>
            <span className="text-slate-500">•</span>
            <span className="text-slate-400">Assets - Claims = 0</span>
          </div>
        </div>

        {/* Total Assets */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 shadow-sm">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Assets</span>
            <ArrowUpRight className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="text-2xl font-bold text-white tracking-tight">
            {formatINR(totalAssets)}
          </div>
          <p className="mt-2 text-xs text-slate-400">Bank, Demat, Cash Wallets</p>
        </div>

        {/* Total Liabilities */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 shadow-sm">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Liabilities</span>
            <ArrowDownRight className="w-4 h-4 text-rose-400" />
          </div>
          <div className="text-2xl font-bold text-white tracking-tight">
            {formatINR(totalLiabilities)}
          </div>
          <p className="mt-2 text-xs text-slate-400">Credit cards, payables, loans</p>
        </div>

        {/* Monthly Net Savings / Cash Flow */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 shadow-sm">
          <div className="flex items-center justify-between text-slate-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">30-Day Net Cash Flow</span>
            <TrendingUp className="w-4 h-4 text-blue-400" />
          </div>
          <div className={`text-2xl font-bold tracking-tight ${monthlyProfit >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
            {formatINR(monthlyProfit)}
          </div>
          <p className="mt-2 text-xs text-slate-400">Income - Operating Expenses</p>
        </div>
      </div>

      {/* Middle Row: Trading Snapshot & Asset Split */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Trading Snapshot */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 lg:col-span-2">
          <div className="flex items-center justify-between mb-6">
            <div>
              <h2 className="text-base font-bold text-white">Trading Journal Performance</h2>
              <p className="text-xs text-slate-400">Audited P&L across all broker execution segments</p>
            </div>
            <button
              onClick={() => onNavigate('trading')}
              className="text-xs text-emerald-400 hover:text-emerald-300 flex items-center space-x-1"
            >
              <span>View Full Journal</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div className="p-4 rounded-lg bg-slate-900/60 border border-slate-800">
              <span className="text-xs text-slate-400 block mb-1">Win Rate</span>
              <span className="text-xl font-bold text-white">{formatBps(winRate)}</span>
              <span className="text-[10px] text-slate-500 block mt-1">
                {tradingSummary?.winning_trades} wins / {tradingSummary?.total_trades} trades
              </span>
            </div>

            <div className="p-4 rounded-lg bg-slate-900/60 border border-slate-800">
              <span className="text-xs text-slate-400 block mb-1">Net Realized P&L</span>
              <span className={`text-xl font-bold ${tradingNet >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                {formatINR(tradingNet)}
              </span>
              <span className="text-[10px] text-slate-500 block mt-1">
                Charges: {formatINR(tradingSummary?.total_charges_paisa ?? 0)}
              </span>
            </div>

            <div className="p-4 rounded-lg bg-slate-900/60 border border-slate-800">
              <span className="text-xs text-slate-400 block mb-1">Profit Factor</span>
              <span className="text-xl font-bold text-white">
                {((tradingSummary?.profit_factor_scaled ?? 0) / 100).toFixed(2)}x
              </span>
              <span className="text-[10px] text-slate-500 block mt-1">Gross Profit / Loss</span>
            </div>
          </div>
        </div>

        {/* Quick Balance Breakdown */}
        <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-base font-bold text-white">Asset Allocation</h2>
              <Layers className="w-4 h-4 text-slate-400" />
            </div>
            <div className="space-y-3">
              {balanceSheet?.assets.map((a) => (
                <div key={a.account_id} className="flex items-center justify-between text-xs">
                  <span className="text-slate-300 font-medium">{a.name}</span>
                  <span className="text-white font-mono">{formatINR(a.balance_paisa)}</span>
                </div>
              ))}
            </div>
          </div>

          <div className="pt-4 border-t border-slate-800/80 mt-4">
            <button
              onClick={() => onNavigate('assets_liabilities')}
              className="w-full py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg transition"
            >
              Inspect All Assets & Liabilities
            </button>
          </div>
        </div>
      </div>

      {/* Recent Transactions Table */}
      <div className="p-6 rounded-xl bg-[#111726] border border-slate-800">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-base font-bold text-white">Recent Ledger Transactions</h2>
            <p className="text-xs text-slate-400">Strictly balanced double-entry immutable records</p>
          </div>
          <div className="flex items-center space-x-3">
            <button
              onClick={onOpenTxModal}
              className="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-md transition"
            >
              + Post Entry
            </button>
            <button
              onClick={() => onNavigate('transactions')}
              className="text-xs text-slate-400 hover:text-white"
            >
              View All
            </button>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-slate-800 text-slate-400 pb-3 font-semibold uppercase tracking-wider">
                <th className="pb-3">Date</th>
                <th className="pb-3">Type</th>
                <th className="pb-3">Category</th>
                <th className="pb-3">Reference / Notes</th>
                <th className="pb-3">Status</th>
                <th className="pb-3 text-right">Balanced Postings</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {recentTxs.map((tx) => {
                const totalAmount = tx.postings[0]?.amount_paisa ?? 0;
                return (
                  <tr key={tx.id} className="hover:bg-slate-850 transition">
                    <td className="py-3 text-slate-300">{formatDate(tx.timestamp)}</td>
                    <td className="py-3">
                      <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-slate-800 text-slate-200 border border-slate-700">
                        {tx.tx_type}
                      </span>
                    </td>
                    <td className="py-3 text-slate-300">{tx.category || 'General'}</td>
                    <td className="py-3 text-slate-400 font-mono text-[11px]">
                      {tx.reference_number || tx.notes || '—'}
                    </td>
                    <td className="py-3">
                      <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                        {tx.verification_status}
                      </span>
                    </td>
                    <td className="py-3 text-right font-mono font-semibold text-white">
                      {formatINR(totalAmount)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
