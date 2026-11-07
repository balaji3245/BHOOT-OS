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
  ShieldCheck,
  Plus,
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
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Decrypting & Loading Ledger...
        </div>
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
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] shadow-sm relative overflow-hidden">
          <div className="flex items-center justify-between text-neutral-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Net Worth</span>
            <Wallet className="w-4 h-4 text-white" />
          </div>
          <div className="text-3xl font-extrabold text-white tracking-tight font-mono">
            {formatINR(netWorth)}
          </div>
          <div className="mt-3 flex items-center space-x-2 text-xs">
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
            <span className="text-emerald-400 font-medium">Equilibrium Verified</span>
            <span className="text-neutral-600">•</span>
            <span className="text-neutral-400">Assets = Equity</span>
          </div>
        </div>

        {/* Total Assets */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] shadow-sm">
          <div className="flex items-center justify-between text-neutral-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Assets</span>
            <ArrowUpRight className="w-4 h-4 text-neutral-400" />
          </div>
          <div className="text-2xl font-bold text-white tracking-tight font-mono">
            {formatINR(totalAssets)}
          </div>
          <p className="mt-2 text-xs text-neutral-400">
            {balanceSheet?.assets.length || 0} active accounts (Bank, Demat, Cash)
          </p>
        </div>

        {/* Total Liabilities */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] shadow-sm">
          <div className="flex items-center justify-between text-neutral-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">Total Liabilities</span>
            <ArrowDownRight className="w-4 h-4 text-neutral-400" />
          </div>
          <div className="text-2xl font-bold text-white tracking-tight font-mono">
            {formatINR(totalLiabilities)}
          </div>
          <p className="mt-2 text-xs text-neutral-400">
            {balanceSheet?.liabilities.length || 0} claims (Credit cards, payables)
          </p>
        </div>

        {/* Monthly Net Savings / Cash Flow */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] shadow-sm">
          <div className="flex items-center justify-between text-neutral-400 mb-2">
            <span className="text-xs uppercase tracking-wider font-semibold">30-Day Net Cash Flow</span>
            <TrendingUp className="w-4 h-4 text-neutral-400" />
          </div>
          <div
            className={`text-2xl font-bold tracking-tight font-mono ${
              monthlyProfit >= 0 ? 'text-emerald-400' : 'text-red-400'
            }`}
          >
            {formatINR(monthlyProfit)}
          </div>
          <p className="mt-2 text-xs text-neutral-400">Operating Net Savings</p>
        </div>
      </div>

      {/* Middle Row: Trading Snapshot & Asset Allocation */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Trading Snapshot */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] lg:col-span-2">
          <div className="flex items-center justify-between mb-6">
            <div>
              <h2 className="text-base font-bold text-white">Trading Performance</h2>
              <p className="text-xs text-neutral-400">Audited P&L across all broker execution segments</p>
            </div>
            <button
              onClick={() => onNavigate('trading')}
              className="text-xs text-white hover:text-neutral-300 flex items-center space-x-1 font-medium transition"
            >
              <span>View Full Journal</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div className="p-4 rounded-lg bg-[#181818] border border-[#222222]">
              <span className="text-xs text-neutral-400 block mb-1">Win Rate</span>
              <span className="text-xl font-bold text-white font-mono">{formatBps(winRate)}</span>
              <span className="text-[10px] text-neutral-500 block mt-1">
                {tradingSummary?.winning_trades ?? 0} wins / {tradingSummary?.total_trades ?? 0} trades
              </span>
            </div>

            <div className="p-4 rounded-lg bg-[#181818] border border-[#222222]">
              <span className="text-xs text-neutral-400 block mb-1">Net Realized P&L</span>
              <span
                className={`text-xl font-bold font-mono ${
                  tradingNet >= 0 ? 'text-emerald-400' : 'text-red-400'
                }`}
              >
                {formatINR(tradingNet)}
              </span>
              <span className="text-[10px] text-neutral-500 block mt-1">
                Statutory charges: {formatINR(tradingSummary?.total_charges_paisa ?? 0)}
              </span>
            </div>

            <div className="p-4 rounded-lg bg-[#181818] border border-[#222222]">
              <span className="text-xs text-neutral-400 block mb-1">Profit Factor</span>
              <span className="text-xl font-bold text-white font-mono">
                {((tradingSummary?.profit_factor_scaled ?? 0) / 100).toFixed(2)}x
              </span>
              <span className="text-[10px] text-neutral-500 block mt-1">Gross Profit / Gross Loss</span>
            </div>
          </div>
        </div>

        {/* Quick Balance Breakdown */}
        <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-base font-bold text-white">Asset Allocation</h2>
              <Layers className="w-4 h-4 text-neutral-400" />
            </div>
            <div className="space-y-3">
              {balanceSheet?.assets.slice(0, 4).map((a) => (
                <div key={a.account_id} className="flex items-center justify-between text-xs">
                  <span className="text-neutral-300 font-medium truncate max-w-[160px]">{a.name}</span>
                  <span className="text-white font-mono">{formatINR(a.balance_paisa)}</span>
                </div>
              ))}
              {(!balanceSheet?.assets || balanceSheet.assets.length === 0) && (
                <div className="text-xs text-neutral-500 py-3 text-center">No asset accounts recorded</div>
              )}
            </div>
          </div>

          <div className="pt-4 border-t border-[#222222] mt-4">
            <button
              onClick={() => onNavigate('assets_liabilities')}
              className="w-full py-2 bg-[#181818] hover:bg-[#222222] text-white text-xs font-semibold rounded-lg border border-[#333333] transition"
            >
              Inspect All Assets & Liabilities
            </button>
          </div>
        </div>
      </div>

      {/* Recent Transactions Table */}
      <div className="p-6 rounded-xl bg-[#111111] border border-[#222222]">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-base font-bold text-white">Recent Ledger Transactions</h2>
            <p className="text-xs text-neutral-400">Strictly balanced double-entry immutable records</p>
          </div>
          <div className="flex items-center space-x-3">
            <button
              onClick={onOpenTxModal}
              className="px-3.5 py-1.5 bg-white hover:bg-neutral-200 text-black text-xs font-semibold rounded-lg transition flex items-center space-x-1.5"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Post Entry</span>
            </button>
            <button
              onClick={() => onNavigate('transactions')}
              className="text-xs text-neutral-400 hover:text-white transition"
            >
              View All
            </button>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-[#222222] text-neutral-400 pb-3 font-semibold uppercase tracking-wider">
                <th className="pb-3 px-3">Date</th>
                <th className="pb-3 px-3">Type</th>
                <th className="pb-3 px-3">Category</th>
                <th className="pb-3 px-3">Reference / Notes</th>
                <th className="pb-3 px-3">Status</th>
                <th className="pb-3 px-3 text-right">Balanced Postings</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#222222]/80">
              {recentTxs.map((tx) => {
                const totalAmount = tx.postings[0]?.amount_paisa ?? 0;
                return (
                  <tr key={tx.id} className="hover:bg-[#161616] transition">
                    <td className="py-3 px-3 text-neutral-300 font-mono">{formatDate(tx.timestamp)}</td>
                    <td className="py-3 px-3">
                      <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-[#181818] text-white border border-[#333333]">
                        {tx.tx_type}
                      </span>
                    </td>
                    <td className="py-3 px-3 text-neutral-300">{tx.category || 'General'}</td>
                    <td className="py-3 px-3 text-neutral-400 font-mono text-[11px]">
                      {tx.reference_number || tx.notes || '—'}
                    </td>
                    <td className="py-3 px-3">
                      <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                        {tx.verification_status}
                      </span>
                    </td>
                    <td className="py-3 px-3 text-right font-mono font-semibold text-white">
                      {formatINR(totalAmount)}
                    </td>
                  </tr>
                );
              })}
              {recentTxs.length === 0 && (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-xs text-neutral-500">
                    No transactions recorded in the ledger yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
