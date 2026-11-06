import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Account, TradeRecord, TradeType, TradingSummary } from '../types';
import { formatINR, formatBps, formatDate } from '../utils/format';
import { Plus, Download, X } from 'lucide-react';

export const TradingJournalView: React.FC = () => {
  const [trades, setTrades] = useState<TradeRecord[]>([]);
  const [summary, setSummary] = useState<TradingSummary | null>(null);
  const [brokers, setBrokers] = useState<Account[]>([]);
  const [selectedBroker, setSelectedBroker] = useState<string>('');
  const [modalOpen, setModalOpen] = useState(false);

  // New Trade Form
  const [symbol, setSymbol] = useState('');
  const [tradeType, setTradeType] = useState<TradeType>('Options');
  const [grossRupees, setGrossRupees] = useState('');
  const [chargesRupees, setChargesRupees] = useState('');
  const [orderId, setOrderId] = useState('');

  const loadData = async (bId?: string) => {
    try {
      const trs = await api.getTrades(bId);
      const sum = await api.getTradingSummary(bId);
      const accs = await api.getAccounts();
      const brokerAccs = accs.filter((a) => a.subtype === 'BrokerAccount');

      setTrades(trs);
      setSummary(sum);
      setBrokers(brokerAccs);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData(selectedBroker || undefined);
  }, [selectedBroker]);

  const handleRecordTrade = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!symbol.trim() || !grossRupees || !chargesRupees) return;

    try {
      const brokerId = selectedBroker || brokers[0]?.id;
      if (!brokerId) {
        alert('Please create or select a broker account first');
        return;
      }

      const grossPaisa = Math.round(parseFloat(grossRupees) * 100);
      const chargesPaisa = Math.round(parseFloat(chargesRupees) * 100);
      const netPaisa = grossPaisa - chargesPaisa;

      await api.recordTrade({
        broker_account_id: brokerId,
        trade_date: new Date().toISOString(),
        symbol: symbol.trim().toUpperCase(),
        trade_type: tradeType,
        gross_pnl_paisa: grossPaisa,
        total_charges_paisa: chargesPaisa,
        net_pnl_paisa: netPaisa,
        order_id: orderId.trim() || undefined,
      });

      setModalOpen(false);
      setSymbol('');
      setGrossRupees('');
      setChargesRupees('');
      setOrderId('');
      loadData(selectedBroker || undefined);
    } catch (err: any) {
      alert(`Failed to record trade: ${err.message}`);
    }
  };

  const handleExportTradesCsv = () => {
    let csv =
      '# WARNING: UNENCRYPTED FINANCIAL RECORD EXPORTED FROM BHOOT OS\nTradeID,Date,Symbol,Type,GrossPnL_INR,Charges_INR,NetPnL_INR,OrderID\n';
    trades.forEach((t) => {
      csv += `"${t.id}","${t.trade_date}","${t.symbol}","${t.trade_type}",${
        t.gross_pnl_paisa / 100
      },${t.total_charges_paisa / 100},${t.net_pnl_paisa / 100},"${t.order_id || ''}"\n`;
    });
    api.exportCsv(csv, `bhoot_trades_${Date.now()}.csv`);
  };

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Trading Journal</h1>
          <p className="text-xs text-slate-400">
            F&O and equity trade recordkeeping with statutory fees and net P&L metrics
          </p>
        </div>

        <div className="flex items-center space-x-3">
          {brokers.length > 0 && (
            <select
              value={selectedBroker}
              onChange={(e) => setSelectedBroker(e.target.value)}
              className="bg-slate-900 border border-slate-700 text-xs rounded-lg px-3 py-2 text-slate-200"
            >
              <option value="">All Brokers</option>
              {brokers.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.name}
                </option>
              ))}
            </select>
          )}

          <button
            onClick={handleExportTradesCsv}
            className="flex items-center space-x-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg border border-slate-700 transition"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Export Trades</span>
          </button>

          <button
            onClick={() => setModalOpen(true)}
            className="flex items-center space-x-1.5 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition"
          >
            <Plus className="w-4 h-4" />
            <span>Log Trade</span>
          </button>
        </div>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="p-5 rounded-xl bg-[#111726] border border-slate-800">
          <span className="text-[10px] uppercase font-semibold text-slate-400 block mb-1">
            Net Realized P&L
          </span>
          <span
            className={`text-2xl font-bold font-mono ${
              (summary?.net_pnl_paisa || 0) >= 0 ? 'text-emerald-400' : 'text-rose-400'
            }`}
          >
            {formatINR(summary?.net_pnl_paisa || 0)}
          </span>
          <span className="text-[10px] text-slate-500 block mt-1">
            Charges: {formatINR(summary?.total_charges_paisa || 0)}
          </span>
        </div>

        <div className="p-5 rounded-xl bg-[#111726] border border-slate-800">
          <span className="text-[10px] uppercase font-semibold text-slate-400 block mb-1">Win Rate</span>
          <span className="text-2xl font-bold font-mono text-white">
            {formatBps(summary?.win_rate_basis_points || 0)}
          </span>
          <span className="text-[10px] text-slate-500 block mt-1">
            {summary?.winning_trades} wins / {summary?.losing_trades} losses
          </span>
        </div>

        <div className="p-5 rounded-xl bg-[#111726] border border-slate-800">
          <span className="text-[10px] uppercase font-semibold text-slate-400 block mb-1">
            Profit Factor
          </span>
          <span className="text-2xl font-bold font-mono text-white">
            {((summary?.profit_factor_scaled || 0) / 100).toFixed(2)}x
          </span>
          <span className="text-[10px] text-slate-500 block mt-1">Gross Win / Gross Loss</span>
        </div>

        <div className="p-5 rounded-xl bg-[#111726] border border-slate-800">
          <span className="text-[10px] uppercase font-semibold text-slate-400 block mb-1">
            Total Trades
          </span>
          <span className="text-2xl font-bold font-mono text-white">
            {summary?.total_trades || 0}
          </span>
          <span className="text-[10px] text-slate-500 block mt-1">Audited executions</span>
        </div>
      </div>

      {/* Trades Table */}
      <div className="bg-[#111726] rounded-xl border border-slate-800 overflow-hidden shadow-sm">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/50 text-slate-400 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">Date</th>
                <th className="py-3 px-4">Symbol</th>
                <th className="py-3 px-4">Segment</th>
                <th className="py-3 px-4 text-right">Gross P&L</th>
                <th className="py-3 px-4 text-right">Regulatory Charges</th>
                <th className="py-3 px-4 text-right">Net P&L</th>
                <th className="py-3 px-4">Order ID</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60 font-mono">
              {trades.map((t) => {
                const isNetWin = t.net_pnl_paisa >= 0;
                return (
                  <tr key={t.id} className="hover:bg-slate-850/60 transition">
                    <td className="py-3 px-4 text-slate-300 font-sans whitespace-nowrap">
                      {formatDate(t.trade_date)}
                    </td>
                    <td className="py-3 px-4 font-bold text-white font-sans">{t.symbol}</td>
                    <td className="py-3 px-4 font-sans">
                      <span className="px-2 py-0.5 rounded text-[10px] bg-slate-800 text-slate-300 border border-slate-700">
                        {t.trade_type}
                      </span>
                    </td>
                    <td
                      className={`py-3 px-4 text-right font-bold ${
                        t.gross_pnl_paisa >= 0 ? 'text-emerald-400' : 'text-rose-400'
                      }`}
                    >
                      {formatINR(t.gross_pnl_paisa)}
                    </td>
                    <td className="py-3 px-4 text-right text-slate-400">
                      {formatINR(t.total_charges_paisa)}
                    </td>
                    <td
                      className={`py-3 px-4 text-right font-bold ${
                        isNetWin ? 'text-emerald-400' : 'text-rose-400'
                      }`}
                    >
                      {formatINR(t.net_pnl_paisa)}
                    </td>
                    <td className="py-3 px-4 text-slate-500 text-[11px] font-sans">
                      {t.order_id || '—'}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal: Log Trade */}
      {modalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <div className="bg-[#111726] border border-slate-800 rounded-xl max-w-md w-full p-6 space-y-4">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-white">Log Trading Execution</h3>
              <button onClick={() => setModalOpen(false)} className="text-slate-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleRecordTrade} className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">Symbol</label>
                  <input
                    type="text"
                    placeholder="e.g. NIFTY26NOV24500CE"
                    value={symbol}
                    onChange={(e) => setSymbol(e.target.value)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 uppercase font-mono"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">Segment</label>
                  <select
                    value={tradeType}
                    onChange={(e) => setTradeType(e.target.value as TradeType)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                  >
                    <option value="Options">Options</option>
                    <option value="Futures">Futures</option>
                    <option value="IntradayEquity">Intraday Equity</option>
                    <option value="DeliveryEquity">Delivery Equity</option>
                    <option value="Commodity">Commodity</option>
                    <option value="Currency">Currency</option>
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">
                    Gross P&L (₹)
                  </label>
                  <input
                    type="number"
                    step="0.01"
                    placeholder="e.g. 5000 or -2500"
                    value={grossRupees}
                    onChange={(e) => setGrossRupees(e.target.value)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 font-mono text-right"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">
                    Charges (STT/GST) (₹)
                  </label>
                  <input
                    type="number"
                    step="0.01"
                    min="0"
                    placeholder="e.g. 240.50"
                    value={chargesRupees}
                    onChange={(e) => setChargesRupees(e.target.value)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 font-mono text-right"
                    required
                  />
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-300 mb-1">
                  Broker Order ID (Optional)
                </label>
                <input
                  type="text"
                  placeholder="e.g. ORD20261106001"
                  value={orderId}
                  onChange={(e) => setOrderId(e.target.value)}
                  className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 font-mono"
                />
              </div>

              {/* Net P&L Preview */}
              {grossRupees && chargesRupees && (
                <div className="p-3 bg-slate-900/60 rounded-lg border border-slate-800 flex justify-between text-xs">
                  <span className="text-slate-400">Calculated Net P&L:</span>
                  <span className="font-mono font-bold text-emerald-400">
                    {formatINR(
                      Math.round(parseFloat(grossRupees) * 100) -
                        Math.round(parseFloat(chargesRupees) * 100)
                    )}
                  </span>
                </div>
              )}

              <div className="flex justify-end space-x-3 pt-3 border-t border-slate-800">
                <button
                  type="button"
                  onClick={() => setModalOpen(false)}
                  className="px-4 py-2 bg-slate-800 text-slate-300 text-xs font-semibold rounded-lg"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm"
                >
                  Record Trade
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
