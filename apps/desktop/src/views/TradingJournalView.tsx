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
  const [loading, setLoading] = useState(true);

  // New Trade Form
  const [symbol, setSymbol] = useState('');
  const [tradeType, setTradeType] = useState<TradeType>('Options');
  const [grossRupees, setGrossRupees] = useState('');
  const [brokerageRupees, setBrokerageRupees] = useState('40.00');
  const [sttRupees, setSttRupees] = useState('50.00');
  const [exchangeRupees, setExchangeRupees] = useState('15.00');
  const [gstRupees, setGstRupees] = useState('10.00');
  const [sebiRupees, setSebiRupees] = useState('1.00');
  const [stampDutyRupees, setStampDutyRupees] = useState('3.00');
  const [orderId, setOrderId] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const loadData = async () => {
    try {
      const trs = await api.getTrades();
      const sum = await api.getTradingSummary();
      const accs = await api.getAccounts();
      const brokerAccs = accs.filter((a) => a.subtype === 'BrokerAccount' || a.account_type === 'Asset');

      setTrades(trs);
      setSummary(sum);
      setBrokers(brokerAccs);
    } catch (err) {
      console.error('Failed to load trading journal:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const totalChargesPaisa =
    Math.round((parseFloat(brokerageRupees) || 0) * 100) +
    Math.round((parseFloat(sttRupees) || 0) * 100) +
    Math.round((parseFloat(exchangeRupees) || 0) * 100) +
    Math.round((parseFloat(gstRupees) || 0) * 100) +
    Math.round((parseFloat(sebiRupees) || 0) * 100) +
    Math.round((parseFloat(stampDutyRupees) || 0) * 100);

  const grossPaisa = Math.round((parseFloat(grossRupees) || 0) * 100);
  const calculatedNetPaisa = grossPaisa - totalChargesPaisa;

  const handleRecordTrade = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!symbol.trim() || !grossRupees) return;

    setIsSubmitting(true);
    try {
      const brokerId = selectedBroker || brokers[0]?.id;
      if (!brokerId) {
        alert('Please create or select an asset/broker account first in Chart of Accounts');
        return;
      }

      await api.recordTrade({
        broker_account_id: brokerId,
        trade_date: new Date().toISOString(),
        symbol: symbol.trim().toUpperCase(),
        trade_type: tradeType,
        gross_amount_paisa: grossPaisa,
        charges: {
          brokerage_paisa: Math.round((parseFloat(brokerageRupees) || 0) * 100),
          stt_paisa: Math.round((parseFloat(sttRupees) || 0) * 100),
          exchange_turnover_paisa: Math.round((parseFloat(exchangeRupees) || 0) * 100),
          gst_paisa: Math.round((parseFloat(gstRupees) || 0) * 100),
          sebi_paisa: Math.round((parseFloat(sebiRupees) || 0) * 100),
          stamp_duty_paisa: Math.round((parseFloat(stampDutyRupees) || 0) * 100),
        },
        net_amount_paisa: calculatedNetPaisa,
        order_reference: orderId.trim() || undefined,
      });

      setModalOpen(false);
      setSymbol('');
      setGrossRupees('');
      setOrderId('');
      await loadData();
    } catch (err: any) {
      alert(`Failed to record trade: ${err.message}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleExportTradesCsv = () => {
    let csv =
      '# WARNING: UNENCRYPTED FINANCIAL RECORD EXPORTED FROM BHOOT OS\nTradeID,Date,Symbol,Type,GrossPnL_INR,Charges_INR,NetPnL_INR,OrderID\n';
    trades.forEach((t: any) => {
      const gross = (t.gross_pnl ?? t.gross_pnl_paisa ?? 0) / 100;
      const charges =
        t.charges && typeof t.charges === 'object'
          ? (Object.values(t.charges).reduce((a: any, b: any) => (a as number) + (b as number), 0) as number) / 100
          : (t.total_charges_paisa ?? 0) / 100;
      const net = (t.net_pnl ?? t.net_pnl_paisa ?? 0) / 100;

      csv += `"${t.id}","${t.trade_date}","${t.symbol}","${t.trade_type}",${gross},${charges},${net},"${t.order_id || ''}"\n`;
    });
    api.exportCsv(csv, `bhoot_trades_${Date.now()}.csv`);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Loading Trading Journal & Performance...
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Trading Journal</h1>
          <p className="text-xs text-neutral-400">
            F&O and equity trade recordkeeping with statutory fees and net P&L metrics
          </p>
        </div>

        <div className="flex items-center space-x-3">
          {brokers.length > 0 && (
            <select
              value={selectedBroker}
              onChange={(e) => setSelectedBroker(e.target.value)}
              className="bg-[#181818] border border-[#333333] text-xs rounded-lg px-3 py-2 text-white focus:border-white focus:outline-none"
            >
              <option value="">All Accounts</option>
              {brokers.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.name}
                </option>
              ))}
            </select>
          )}

          <button
            onClick={handleExportTradesCsv}
            className="flex items-center space-x-1.5 px-3 py-2 bg-[#181818] hover:bg-[#222222] text-white text-xs font-semibold rounded-lg border border-[#333333] transition"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Export Trades</span>
          </button>

          <button
            onClick={() => setModalOpen(true)}
            className="flex items-center space-x-1.5 px-4 py-2 bg-white hover:bg-neutral-200 text-black text-xs font-semibold rounded-lg shadow-sm transition"
          >
            <Plus className="w-4 h-4" />
            <span>Log Trade</span>
          </button>
        </div>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="p-5 rounded-xl bg-[#111111] border border-[#222222]">
          <span className="text-[10px] uppercase font-semibold text-neutral-400 block mb-1">
            Net Realized P&L
          </span>
          <span
            className={`text-2xl font-bold font-mono ${
              (summary?.net_pnl_paisa || 0) >= 0 ? 'text-emerald-400' : 'text-red-400'
            }`}
          >
            {formatINR(summary?.net_pnl_paisa || 0)}
          </span>
          <span className="text-[10px] text-neutral-500 block mt-1 font-mono">
            Charges: {formatINR(summary?.total_charges_paisa || 0)}
          </span>
        </div>

        <div className="p-5 rounded-xl bg-[#111111] border border-[#222222]">
          <span className="text-[10px] uppercase font-semibold text-neutral-400 block mb-1">Win Rate</span>
          <span className="text-2xl font-bold font-mono text-white">
            {formatBps(summary?.win_rate_basis_points || 0)}
          </span>
          <span className="text-[10px] text-neutral-500 block mt-1">
            {summary?.winning_trades || 0} wins / {summary?.losing_trades || 0} losses
          </span>
        </div>

        <div className="p-5 rounded-xl bg-[#111111] border border-[#222222]">
          <span className="text-[10px] uppercase font-semibold text-neutral-400 block mb-1">
            Profit Factor
          </span>
          <span className="text-2xl font-bold font-mono text-white">
            {((summary?.profit_factor_scaled || 0) / 100).toFixed(2)}x
          </span>
          <span className="text-[10px] text-neutral-500 block mt-1">
            {summary?.total_trades || 0} executed trades
          </span>
        </div>

        <div className="p-5 rounded-xl bg-[#111111] border border-[#222222]">
          <span className="text-[10px] uppercase font-semibold text-neutral-400 block mb-1">
            Gross Performance
          </span>
          <span className="text-sm font-bold font-mono text-white block">
            Profit: {formatINR(summary?.gross_profit_paisa || 0)}
          </span>
          <span className="text-sm font-bold font-mono text-neutral-400 block mt-0.5">
            Loss: {formatINR(summary?.gross_loss_paisa || 0)}
          </span>
        </div>
      </div>

      {/* Trades Table */}
      <div className="bg-[#111111] rounded-xl border border-[#222222] overflow-hidden shadow-sm">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-[#222222] bg-[#141414] text-neutral-400 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">Date</th>
                <th className="py-3 px-4">Symbol</th>
                <th className="py-3 px-4">Segment</th>
                <th className="py-3 px-4">Order ID / Ref</th>
                <th className="py-3 px-4 text-right">Gross P&L</th>
                <th className="py-3 px-4 text-right">Charges</th>
                <th className="py-3 px-4 text-right">Net P&L</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#222222]/80">
              {trades.map((t: any) => {
                const gross = t.gross_pnl ?? t.gross_pnl_paisa ?? 0;
                const totalCharges =
                  t.charges && typeof t.charges === 'object'
                    ? Object.values(t.charges).reduce((a: any, b: any) => (a as number) + (b as number), 0)
                    : t.total_charges_paisa ?? 0;
                const net = t.net_pnl ?? t.net_pnl_paisa ?? 0;

                return (
                  <tr key={t.id} className="hover:bg-[#161616] transition">
                    <td className="py-3 px-4 text-neutral-300 font-mono whitespace-nowrap">
                      {formatDate(t.trade_date)}
                    </td>
                    <td className="py-3 px-4 font-bold text-white font-mono">{t.symbol}</td>
                    <td className="py-3 px-4">
                      <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-[#181818] text-white border border-[#333333]">
                        {t.trade_type}
                      </span>
                    </td>
                    <td className="py-3 px-4 font-mono text-neutral-400 text-[11px]">
                      {t.order_id || '—'}
                    </td>
                    <td className="py-3 px-4 text-right font-mono text-neutral-300">
                      {formatINR(gross)}
                    </td>
                    <td className="py-3 px-4 text-right font-mono text-neutral-400">
                      {formatINR(totalCharges as number)}
                    </td>
                    <td
                      className={`py-3 px-4 text-right font-mono font-bold whitespace-nowrap ${
                        net >= 0 ? 'text-emerald-400' : 'text-red-400'
                      }`}
                    >
                      {formatINR(net)}
                    </td>
                  </tr>
                );
              })}
              {trades.length === 0 && (
                <tr>
                  <td colSpan={7} className="py-8 text-center text-xs text-neutral-500">
                    No trade executions logged in the journal yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal: New Trade */}
      {modalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4 overflow-y-auto">
          <div className="bg-[#111111] border border-[#222222] rounded-xl max-w-lg w-full p-6 space-y-4 my-8 shadow-2xl">
            <div className="flex items-center justify-between border-b border-[#222222] pb-3">
              <h3 className="text-base font-bold text-white">Log Trading Journal Record</h3>
              <button onClick={() => setModalOpen(false)} className="text-neutral-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleRecordTrade} className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Contract / Symbol</label>
                  <input
                    type="text"
                    placeholder="e.g. NIFTY26NOV24500CE"
                    value={symbol}
                    onChange={(e) => setSymbol(e.target.value)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white font-mono uppercase focus:border-white focus:outline-none"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Execution Segment</label>
                  <select
                    value={tradeType}
                    onChange={(e) => setTradeType(e.target.value as TradeType)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
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
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">
                    Gross P&L (₹ Rupees)
                  </label>
                  <input
                    type="number"
                    step="0.01"
                    placeholder="e.g. 5200.00 or -1500.00"
                    value={grossRupees}
                    onChange={(e) => setGrossRupees(e.target.value)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white font-mono focus:border-white focus:outline-none"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Order ID / Ref</label>
                  <input
                    type="text"
                    placeholder="e.g. ORD-981248"
                    value={orderId}
                    onChange={(e) => setOrderId(e.target.value)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white font-mono focus:border-white focus:outline-none"
                  />
                </div>
              </div>

              {/* Statutory Charges Breakdown */}
              <div className="p-3 bg-[#181818] rounded-lg border border-[#222222] space-y-2">
                <span className="text-[11px] font-bold text-white uppercase tracking-wider block">
                  Statutory Fees & Taxes Breakdown (₹)
                </span>
                <div className="grid grid-cols-3 gap-2 text-xs">
                  <div>
                    <label className="text-[10px] text-neutral-400 block">Brokerage</label>
                    <input
                      type="number"
                      step="0.01"
                      value={brokerageRupees}
                      onChange={(e) => setBrokerageRupees(e.target.value)}
                      className="w-full bg-[#111111] border border-[#333333] rounded p-1.5 text-white font-mono text-xs focus:border-white focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="text-[10px] text-neutral-400 block">STT / CTT</label>
                    <input
                      type="number"
                      step="0.01"
                      value={sttRupees}
                      onChange={(e) => setSttRupees(e.target.value)}
                      className="w-full bg-[#111111] border border-[#333333] rounded p-1.5 text-white font-mono text-xs focus:border-white focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="text-[10px] text-neutral-400 block">Exchange Turnover</label>
                    <input
                      type="number"
                      step="0.01"
                      value={exchangeRupees}
                      onChange={(e) => setExchangeRupees(e.target.value)}
                      className="w-full bg-[#111111] border border-[#333333] rounded p-1.5 text-white font-mono text-xs focus:border-white focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="text-[10px] text-neutral-400 block">GST (18%)</label>
                    <input
                      type="number"
                      step="0.01"
                      value={gstRupees}
                      onChange={(e) => setGstRupees(e.target.value)}
                      className="w-full bg-[#111111] border border-[#333333] rounded p-1.5 text-white font-mono text-xs focus:border-white focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="text-[10px] text-neutral-400 block">SEBI Charges</label>
                    <input
                      type="number"
                      step="0.01"
                      value={sebiRupees}
                      onChange={(e) => setSebiRupees(e.target.value)}
                      className="w-full bg-[#111111] border border-[#333333] rounded p-1.5 text-white font-mono text-xs focus:border-white focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="text-[10px] text-neutral-400 block">Stamp Duty</label>
                    <input
                      type="number"
                      step="0.01"
                      value={stampDutyRupees}
                      onChange={(e) => setStampDutyRupees(e.target.value)}
                      className="w-full bg-[#111111] border border-[#333333] rounded p-1.5 text-white font-mono text-xs focus:border-white focus:outline-none"
                    />
                  </div>
                </div>

                <div className="pt-2 border-t border-[#222222] flex justify-between text-xs font-mono">
                  <span className="text-neutral-400">Total Statutory Fees:</span>
                  <span className="text-white font-bold">{formatINR(totalChargesPaisa)}</span>
                </div>
              </div>

              {/* Net Result Preview */}
              <div className="p-3 bg-[#111111] border border-[#222222] rounded-lg flex items-center justify-between text-xs font-mono">
                <span className="text-neutral-300">Net Realized Trade P&L:</span>
                <span
                  className={`text-base font-bold ${
                    calculatedNetPaisa >= 0 ? 'text-emerald-400' : 'text-red-400'
                  }`}
                >
                  {formatINR(calculatedNetPaisa)}
                </span>
              </div>

              <div className="flex justify-end space-x-3 pt-3 border-t border-[#222222]">
                <button
                  type="button"
                  onClick={() => setModalOpen(false)}
                  className="px-4 py-2 bg-[#181818] border border-[#333333] text-white text-xs font-semibold rounded-lg hover:bg-[#222222] transition"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={isSubmitting}
                  className="px-4 py-2 bg-white hover:bg-neutral-200 disabled:opacity-50 text-black text-xs font-semibold rounded-lg shadow-sm transition"
                >
                  {isSubmitting ? 'Logging...' : 'Save Trade Record'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
