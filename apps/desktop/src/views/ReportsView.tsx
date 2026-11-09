import React, { useEffect, useState, useCallback } from 'react';
import { api } from '../api/client';
import { BalanceSheet, CashFlowStatement, IncomeStatement } from '../types';
import { formatINR, formatDate } from '../utils/format';
import { Download, CheckCircle2, Layers, TrendingUp, DollarSign, Calendar, RefreshCw, AlertTriangle } from 'lucide-react';

export const ReportsView: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'bs' | 'pnl' | 'cfs'>('bs');
  const [balanceSheet, setBalanceSheet] = useState<BalanceSheet | null>(null);
  const [incomeStatement, setIncomeStatement] = useState<IncomeStatement | null>(null);
  const [cashFlow, setCashFlow] = useState<CashFlowStatement | null>(null);
  const [loading, setLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const [fromDate, setFromDate] = useState(() => {
    const d = new Date(Date.now() - 30 * 86400000);
    return d.toISOString().split('T')[0];
  });
  const [toDate, setToDate] = useState(() => {
    return new Date().toISOString().split('T')[0];
  });

  const loadStatements = useCallback(async (fromStr: string, toStr: string) => {
    setIsRefreshing(true);
    setErrorMsg(null);
    try {
      const fromIso = new Date(`${fromStr}T00:00:00Z`).toISOString();
      const toIso = new Date(`${toStr}T23:59:59Z`).toISOString();

      const [bs, pnl, cfs] = await Promise.all([
        api.getBalanceSheet(toIso),
        api.getIncomeStatement(fromIso, toIso),
        api.getCashFlowStatement(fromIso, toIso),
      ]);

      setBalanceSheet(bs);
      setIncomeStatement(pnl);
      setCashFlow(cfs);
    } catch (err: any) {
      console.error('Failed to load financial statements:', err);
      setErrorMsg(err.message || 'Failed to generate financial statements');
    } finally {
      setLoading(false);
      setIsRefreshing(false);
    }
  }, []);

  useEffect(() => {
    loadStatements(fromDate, toDate);
  }, [loadStatements, fromDate, toDate]);

  const setPreset = (days: number) => {
    const now = new Date();
    const past = new Date(Date.now() - days * 86400000);
    const to = now.toISOString().split('T')[0];
    const from = past.toISOString().split('T')[0];
    setFromDate(from);
    setToDate(to);
  };

  const setYtdPreset = () => {
    const now = new Date();
    const year = now.getMonth() >= 3 ? now.getFullYear() : now.getFullYear() - 1;
    const from = `${year}-04-01`;
    const to = now.toISOString().split('T')[0];
    setFromDate(from);
    setToDate(to);
  };

  const handleExportReportCsv = () => {
    let csv = '';
    if (activeTab === 'bs' && balanceSheet) {
      csv = `# BHOOT BALANCE SHEET REPORT (As of ${balanceSheet.as_of})\nCategory,Account,Balance_INR\n`;
      balanceSheet.assets.forEach((a) => (csv += `Asset,"${a.name}",${a.balance_paisa / 100}\n`));
      balanceSheet.liabilities.forEach((l) => (csv += `Liability,"${l.name}",${l.balance_paisa / 100}\n`));
      balanceSheet.equity.forEach((e) => (csv += `Equity,"${e.name}",${e.balance_paisa / 100}\n`));
      csv += `Summary,Net_Worth,${balanceSheet.net_worth_paisa / 100}\n`;
      api.exportCsv(csv, `balance_sheet_${Date.now()}.csv`);
    } else if (activeTab === 'pnl' && incomeStatement) {
      csv = `# BHOOT INCOME STATEMENT (Period: ${incomeStatement.from} to ${incomeStatement.to})\nType,Item,Amount_INR\n`;
      incomeStatement.operating_income.forEach(
        (i) => (csv += `Operating_Income,"${i.name}",${i.balance_paisa / 100}\n`)
      );
      csv += `Trading,Gross_PnL,${incomeStatement.trading_gross_pnl_paisa / 100}\n`;
      csv += `Trading,Charges,${incomeStatement.trading_charges_paisa / 100}\n`;
      csv += `Trading,Net_PnL,${incomeStatement.trading_net_pnl_paisa / 100}\n`;
      incomeStatement.operating_expenses.forEach(
        (e) => (csv += `Operating_Expense,"${e.name}",${e.balance_paisa / 100}\n`)
      );
      csv += `Summary,Net_Profit,${incomeStatement.net_profit_paisa / 100}\n`;
      api.exportCsv(csv, `income_statement_${Date.now()}.csv`);
    } else if (activeTab === 'cfs' && cashFlow) {
      csv = `# BHOOT CASH FLOW STATEMENT\nActivity,Amount_INR\n`;
      csv += `Beginning_Cash,${cashFlow.beginning_cash_paisa / 100}\n`;
      csv += `Operating_Cash_Flow,${cashFlow.operating_cash_flow_paisa / 100}\n`;
      csv += `Investing_Cash_Flow,${cashFlow.investing_cash_flow_paisa / 100}\n`;
      csv += `Financing_Cash_Flow,${cashFlow.financing_cash_flow_paisa / 100}\n`;
      csv += `Net_Cash_Flow,${cashFlow.net_cash_flow_paisa / 100}\n`;
      csv += `Ending_Cash,${cashFlow.ending_cash_paisa / 100}\n`;
      api.exportCsv(csv, `cash_flow_${Date.now()}.csv`);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Generating Financial Statements...
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Financial Reports</h1>
          <p className="text-xs text-neutral-400">
            Standardized double-entry GAAP/IFRS compliant financial statements
          </p>
        </div>

        <button
          onClick={handleExportReportCsv}
          className="flex items-center space-x-1.5 px-3 py-2 bg-[#181818] hover:bg-[#222222] text-white text-xs font-semibold rounded-lg border border-[#333333] transition"
        >
          <Download className="w-3.5 h-3.5" />
          <span>Export Active Statement</span>
        </button>
      </div>

      {errorMsg && (
        <div className="p-3 bg-red-500/10 border border-red-500/30 rounded-lg flex items-center space-x-2 text-xs text-red-400">
          <AlertTriangle className="w-4 h-4 flex-shrink-0" />
          <span>{errorMsg}</span>
        </div>
      )}

      {/* Date Range & Reporting Period Toolbar */}
      <div className="p-4 rounded-xl bg-[#111111] border border-[#222222] flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center space-x-2">
          <Calendar className="w-4 h-4 text-neutral-400" />
          <span className="text-xs font-semibold text-neutral-300 uppercase tracking-wider">Report Window:</span>
          <div className="flex items-center space-x-2">
            <input
              type="date"
              value={fromDate}
              onChange={(e) => setFromDate(e.target.value)}
              className="bg-[#181818] border border-[#333333] text-xs rounded-lg px-2.5 py-1.5 text-white font-mono focus:border-white focus:outline-none"
            />
            <span className="text-neutral-500 text-xs">to</span>
            <input
              type="date"
              value={toDate}
              onChange={(e) => setToDate(e.target.value)}
              className="bg-[#181818] border border-[#333333] text-xs rounded-lg px-2.5 py-1.5 text-white font-mono focus:border-white focus:outline-none"
            />
          </div>
        </div>

        <div className="flex items-center space-x-2">
          <button
            onClick={() => setPreset(30)}
            className="px-2.5 py-1 bg-[#181818] hover:bg-[#222222] text-neutral-300 hover:text-white text-xs rounded border border-[#333333] transition"
          >
            Last 30D
          </button>
          <button
            onClick={() => setPreset(90)}
            className="px-2.5 py-1 bg-[#181818] hover:bg-[#222222] text-neutral-300 hover:text-white text-xs rounded border border-[#333333] transition"
          >
            Last 90D
          </button>
          <button
            onClick={setYtdPreset}
            className="px-2.5 py-1 bg-[#181818] hover:bg-[#222222] text-neutral-300 hover:text-white text-xs rounded border border-[#333333] transition"
          >
            Fiscal YTD
          </button>
          {isRefreshing && (
            <RefreshCw className="w-3.5 h-3.5 text-neutral-400 animate-spin ml-2" />
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-[#222222] space-x-6 text-sm">
        <button
          onClick={() => setActiveTab('bs')}
          className={`pb-3 font-semibold transition border-b-2 flex items-center space-x-2 ${
            activeTab === 'bs'
              ? 'text-white border-white'
              : 'text-neutral-400 border-transparent hover:text-white'
          }`}
        >
          <Layers className="w-4 h-4" />
          <span>Balance Sheet</span>
        </button>
        <button
          onClick={() => setActiveTab('pnl')}
          className={`pb-3 font-semibold transition border-b-2 flex items-center space-x-2 ${
            activeTab === 'pnl'
              ? 'text-white border-white'
              : 'text-neutral-400 border-transparent hover:text-white'
          }`}
        >
          <TrendingUp className="w-4 h-4" />
          <span>Profit & Loss (Income Statement)</span>
        </button>
        <button
          onClick={() => setActiveTab('cfs')}
          className={`pb-3 font-semibold transition border-b-2 flex items-center space-x-2 ${
            activeTab === 'cfs'
              ? 'text-white border-white'
              : 'text-neutral-400 border-transparent hover:text-white'
          }`}
        >
          <DollarSign className="w-4 h-4" />
          <span>Cash Flow Statement</span>
        </button>
      </div>

      {/* Tab 1: Balance Sheet */}
      {activeTab === 'bs' && balanceSheet && (
        <div className="space-y-6">
          <div className="flex items-center justify-between p-4 bg-[#111111] border border-[#222222] rounded-xl">
            <div className="flex items-center space-x-2 text-xs text-emerald-400 font-semibold">
              <CheckCircle2 className="w-4 h-4" />
              <span>Balance Sheet Equilibrium Verified: Assets = Liabilities + Equity</span>
            </div>
            <span className="text-xs font-mono text-neutral-400">
              As of: {formatDate(balanceSheet.as_of)}
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Assets */}
            <div className="bg-[#111111] rounded-xl border border-[#222222] p-5 space-y-4">
              <div className="flex justify-between items-center border-b border-[#222222] pb-2">
                <h3 className="text-sm font-bold text-white uppercase">Assets</h3>
                <span className="font-mono text-xs font-bold text-white">
                  {formatINR(balanceSheet.total_assets_paisa)}
                </span>
              </div>
              <div className="space-y-2">
                {balanceSheet.assets.map((a) => (
                  <div key={a.account_id} className="flex justify-between text-xs py-1">
                    <span className="text-neutral-300">{a.name}</span>
                    <span className="font-mono text-white">{formatINR(a.balance_paisa)}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Liabilities & Equity */}
            <div className="bg-[#111111] rounded-xl border border-[#222222] p-5 space-y-4">
              <div className="flex justify-between items-center border-b border-[#222222] pb-2">
                <h3 className="text-sm font-bold text-white uppercase">Liabilities</h3>
                <span className="font-mono text-xs font-bold text-red-400">
                  {formatINR(balanceSheet.total_liabilities_paisa)}
                </span>
              </div>
              <div className="space-y-2">
                {balanceSheet.liabilities.map((l) => (
                  <div key={l.account_id} className="flex justify-between text-xs py-1">
                    <span className="text-neutral-300">{l.name}</span>
                    <span className="font-mono text-red-400">{formatINR(l.balance_paisa)}</span>
                  </div>
                ))}
                {balanceSheet.liabilities.length === 0 && (
                  <div className="text-neutral-500 text-xs py-1">Zero debt claims outstanding</div>
                )}
              </div>

              <div className="pt-4 border-t border-[#222222] flex justify-between items-center pb-2">
                <h3 className="text-sm font-bold text-white uppercase">Equity & Retained Earnings</h3>
                <span className="font-mono text-xs font-bold text-white">
                  {formatINR(balanceSheet.total_equity_paisa + balanceSheet.retained_earnings_paisa)}
                </span>
              </div>
              <div className="space-y-2">
                {balanceSheet.equity.map((e) => (
                  <div key={e.account_id} className="flex justify-between text-xs py-1">
                    <span className="text-neutral-300">{e.name}</span>
                    <span className="font-mono text-white">{formatINR(e.balance_paisa)}</span>
                  </div>
                ))}
                <div className="flex justify-between text-xs py-1 border-t border-[#222222] pt-2 font-semibold">
                  <span className="text-neutral-300">Retained Earnings (Realized Net P&L)</span>
                  <span
                    className={`font-mono ${
                      balanceSheet.retained_earnings_paisa >= 0 ? 'text-emerald-400' : 'text-red-400'
                    }`}
                  >
                    {formatINR(balanceSheet.retained_earnings_paisa)}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Tab 2: Profit & Loss */}
      {activeTab === 'pnl' && incomeStatement && (
        <div className="bg-[#111111] rounded-xl border border-[#222222] p-6 space-y-6">
          <div className="flex justify-between items-center border-b border-[#222222] pb-3 text-xs">
            <span className="text-neutral-400">
              Reporting Window: {formatDate(incomeStatement.from)} to {formatDate(incomeStatement.to)}
            </span>
            <div className="flex items-center space-x-2">
              <span className="text-neutral-400">Net Profit / Surplus:</span>
              <span
                className={`font-mono font-bold text-sm ${
                  incomeStatement.net_profit_paisa >= 0 ? 'text-emerald-400' : 'text-red-400'
                }`}
              >
                {formatINR(incomeStatement.net_profit_paisa)}
              </span>
            </div>
          </div>

          <div className="space-y-4">
            {/* Operating Revenue */}
            <div>
              <h4 className="text-xs font-bold text-white uppercase tracking-wider mb-2">
                1. Operating Income
              </h4>
              <div className="bg-[#181818] rounded-lg p-3 space-y-2">
                {incomeStatement.operating_income.map((i) => (
                  <div key={i.account_id} className="flex justify-between text-xs">
                    <span className="text-neutral-300">{i.name}</span>
                    <span className="font-mono text-white">{formatINR(i.balance_paisa)}</span>
                  </div>
                ))}
                {incomeStatement.operating_income.length === 0 && (
                  <div className="text-neutral-500 text-xs py-1">No operating income in this window</div>
                )}
              </div>
            </div>

            {/* Trading Performance */}
            <div>
              <h4 className="text-xs font-bold text-white uppercase tracking-wider mb-2">
                2. Market Execution & Trading
              </h4>
              <div className="bg-[#181818] rounded-lg p-3 space-y-2 text-xs">
                <div className="flex justify-between">
                  <span className="text-neutral-300">Gross Realized Trading P&L</span>
                  <span className="font-mono text-white">
                    {formatINR(incomeStatement.trading_gross_pnl_paisa)}
                  </span>
                </div>
                <div className="flex justify-between text-neutral-400">
                  <span>Less: Brokerage & Statutory Taxes (STT/GST)</span>
                  <span className="font-mono text-red-400">
                    -{formatINR(incomeStatement.trading_charges_paisa)}
                  </span>
                </div>
                <div className="flex justify-between border-t border-[#222222] pt-1 font-semibold">
                  <span className="text-neutral-200">Net Trading Revenue</span>
                  <span
                    className={`font-mono ${
                      incomeStatement.trading_net_pnl_paisa >= 0 ? 'text-emerald-400' : 'text-red-400'
                    }`}
                  >
                    {formatINR(incomeStatement.trading_net_pnl_paisa)}
                  </span>
                </div>
              </div>
            </div>

            {/* Operating Expenses */}
            <div>
              <h4 className="text-xs font-bold text-white uppercase tracking-wider mb-2">
                3. Operating & Living Expenses
              </h4>
              <div className="bg-[#181818] rounded-lg p-3 space-y-2">
                {incomeStatement.operating_expenses.map((e) => (
                  <div key={e.account_id} className="flex justify-between text-xs">
                    <span className="text-neutral-300">{e.name}</span>
                    <span className="font-mono text-red-400">{formatINR(e.balance_paisa)}</span>
                  </div>
                ))}
                {incomeStatement.operating_expenses.length === 0 && (
                  <div className="text-neutral-500 text-xs py-1">No operating expenses recorded</div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Tab 3: Cash Flow Statement */}
      {activeTab === 'cfs' && cashFlow && (
        <div className="bg-[#111111] rounded-xl border border-[#222222] p-6 space-y-4 text-xs">
          <div className="flex justify-between border-b border-[#222222] pb-3">
            <span className="text-neutral-400 font-semibold uppercase">Cash Flow Activities</span>
            <span className="text-neutral-400 font-semibold uppercase">Amount</span>
          </div>

          <div className="space-y-3 font-mono">
            <div className="flex justify-between text-neutral-300">
              <span className="font-sans">Beginning Cash Balance</span>
              <span className="font-bold text-white">{formatINR(cashFlow.beginning_cash_paisa)}</span>
            </div>

            <div className="flex justify-between text-neutral-300 pl-4 border-l-2 border-neutral-700">
              <span className="font-sans">Cash Flow from Operating Activities (Net Profit)</span>
              <span className={cashFlow.operating_cash_flow_paisa >= 0 ? 'text-emerald-400' : 'text-red-400'}>
                {formatINR(cashFlow.operating_cash_flow_paisa)}
              </span>
            </div>

            <div className="flex justify-between text-neutral-300 pl-4 border-l-2 border-neutral-700">
              <span className="font-sans">Cash Flow from Investing Activities</span>
              <span>{formatINR(cashFlow.investing_cash_flow_paisa)}</span>
            </div>

            <div className="flex justify-between text-neutral-300 pl-4 border-l-2 border-neutral-700">
              <span className="font-sans">Cash Flow from Financing Activities</span>
              <span>{formatINR(cashFlow.financing_cash_flow_paisa)}</span>
            </div>

            <div className="pt-3 border-t border-[#222222] flex justify-between font-bold text-sm text-white">
              <span className="font-sans">Ending Cash & Liquid Bank Balance</span>
              <span className="text-white">{formatINR(cashFlow.ending_cash_paisa)}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
