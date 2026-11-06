import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Account, Direction, Posting, Transaction, TransactionType } from '../types';
import { formatINR, formatDate } from '../utils/format';
import {
  Plus,
  RotateCcw,
  Download,
  AlertOctagon,
  Search,
  CheckCircle2,
  X,
} from 'lucide-react';

interface TransactionsViewProps {
  txModalOpen: boolean;
  setTxModalOpen: (open: boolean) => void;
}

export const TransactionsView: React.FC<TransactionsViewProps> = ({
  txModalOpen,
  setTxModalOpen,
}) => {
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [search, setSearch] = useState('');
  const [reversalModalTx, setReversalModalTx] = useState<Transaction | null>(null);
  const [reversalReason, setReversalReason] = useState('');
  const [csvWarningOpen, setCsvWarningOpen] = useState(false);

  // New Transaction Form State
  const [txType, setTxType] = useState<TransactionType>('Expense');
  const [category, setCategory] = useState('');
  const [refNumber, setRefNumber] = useState('');
  const [notes, setNotes] = useState('');
  const [postings, setPostings] = useState<
    Array<{ account_id: string; direction: Direction; rupees: string }>
  >([
    { account_id: '', direction: 'Debit', rupees: '' },
    { account_id: '', direction: 'Credit', rupees: '' },
  ]);

  const loadData = async () => {
    try {
      const txs = await api.getTransactions(100, 0);
      const accs = await api.getAccounts();
      setTransactions(txs);
      setAccounts(accs);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  // Postings balance calculation
  const totalDebits = postings
    .filter((p) => p.direction === 'Debit')
    .reduce((sum, p) => sum + Math.round((parseFloat(p.rupees) || 0) * 100), 0);

  const totalCredits = postings
    .filter((p) => p.direction === 'Credit')
    .reduce((sum, p) => sum + Math.round((parseFloat(p.rupees) || 0) * 100), 0);

  const isBalanced = totalDebits > 0 && totalDebits === totalCredits;

  const handleAddPosting = () => {
    setPostings([...postings, { account_id: '', direction: 'Credit', rupees: '' }]);
  };

  const handlePostingChange = (index: number, field: string, val: string) => {
    const updated = [...postings];
    updated[index] = { ...updated[index], [field]: val };
    setPostings(updated);
  };

  const handleSubmitTx = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!isBalanced) return;

    try {
      const validPostings: Posting[] = postings.map((p) => ({
        account_id: p.account_id,
        direction: p.direction,
        amount_paisa: Math.round(parseFloat(p.rupees) * 100),
      }));

      await api.postTransaction({
        tx_type: txType,
        postings: validPostings,
        category: category || undefined,
        reference_number: refNumber || undefined,
        notes: notes || undefined,
      });

      setTxModalOpen(false);
      setPostings([
        { account_id: '', direction: 'Debit', rupees: '' },
        { account_id: '', direction: 'Credit', rupees: '' },
      ]);
      setCategory('');
      setRefNumber('');
      setNotes('');
      loadData();
    } catch (err: any) {
      alert(`Error posting transaction: ${err.message}`);
    }
  };

  const handleExecuteReversal = async () => {
    if (!reversalModalTx || !reversalReason.trim()) return;
    try {
      await api.reverseTransaction(reversalModalTx.id, reversalReason.trim());
      setReversalModalTx(null);
      setReversalReason('');
      loadData();
    } catch (err: any) {
      alert(`Reversal failed: ${err.message}`);
    }
  };

  const handleExportCsv = () => {
    let csv =
      '# WARNING: UNENCRYPTED FINANCIAL RECORD EXPORTED FROM BHOOT OS\n# PROTECT OR SECURELY DESTROY AFTER USE\nTransactionID,Date,Type,Category,Debit_Account,Credit_Account,Amount_INR\n';
    transactions.forEach((t) => {
      const dAcc = accounts.find((a) => a.id === t.postings[0]?.account_id)?.name || '';
      const cAcc = accounts.find((a) => a.id === t.postings[1]?.account_id)?.name || '';
      const amt = (t.postings[0]?.amount_paisa || 0) / 100;
      csv += `"${t.id}","${t.timestamp}","${t.tx_type}","${t.category || ''}","${dAcc}","${cAcc}",${amt}\n`;
    });
    api.exportCsv(csv, `bhoot_transactions_${Date.now()}.csv`);
    setCsvWarningOpen(false);
  };

  const filteredTxs = transactions.filter((t) => {
    const q = search.toLowerCase();
    return (
      t.category?.toLowerCase().includes(q) ||
      t.reference_number?.toLowerCase().includes(q) ||
      t.notes?.toLowerCase().includes(q) ||
      t.tx_type.toLowerCase().includes(q)
    );
  });

  return (
    <div className="p-8 space-y-6 max-w-7xl mx-auto">
      {/* View Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Ledger Transactions</h1>
          <p className="text-xs text-slate-400">
            Strict double-entry journal entries with cryptographic chain integrity
          </p>
        </div>

        <div className="flex items-center space-x-3">
          <button
            onClick={() => setCsvWarningOpen(true)}
            className="flex items-center space-x-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg border border-slate-700 transition"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Export CSV</span>
          </button>

          <button
            onClick={() => setTxModalOpen(true)}
            className="flex items-center space-x-1.5 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition"
          >
            <Plus className="w-4 h-4" />
            <span>New Transaction</span>
          </button>
        </div>
      </div>

      {/* Filter & Search Bar */}
      <div className="flex items-center space-x-3 bg-[#111726] p-3 rounded-lg border border-slate-800">
        <Search className="w-4 h-4 text-slate-400" />
        <input
          type="text"
          placeholder="Search by category, memo, reference ID, or type..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="bg-transparent text-xs text-slate-200 placeholder-slate-500 focus:outline-none flex-1"
        />
      </div>

      {/* Transactions Table */}
      <div className="bg-[#111726] rounded-xl border border-slate-800 overflow-hidden shadow-sm">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/50 text-slate-400 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">Date</th>
                <th className="py-3 px-4">Type</th>
                <th className="py-3 px-4">Category</th>
                <th className="py-3 px-4">Reference / Notes</th>
                <th className="py-3 px-4">Debits & Credits (Legs)</th>
                <th className="py-3 px-4 text-right">Balanced Total</th>
                <th className="py-3 px-4 text-center">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {filteredTxs.map((tx) => {
                const totalAmount = tx.postings[0]?.amount_paisa ?? 0;
                const isReversal = tx.tx_type === 'Reversal';

                return (
                  <tr key={tx.id} className="hover:bg-slate-850/60 transition">
                    <td className="py-3 px-4 text-slate-300 whitespace-nowrap">
                      {formatDate(tx.timestamp)}
                    </td>
                    <td className="py-3 px-4">
                      <span
                        className={`px-2 py-0.5 rounded text-[10px] font-semibold border ${
                          isReversal
                            ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                            : 'bg-slate-800 text-slate-200 border-slate-700'
                        }`}
                      >
                        {tx.tx_type}
                      </span>
                    </td>
                    <td className="py-3 px-4 text-slate-300">{tx.category || '—'}</td>
                    <td className="py-3 px-4 text-slate-400 font-mono text-[11px]">
                      {tx.reference_number || tx.notes || '—'}
                      {tx.reversal_of && (
                        <span className="block text-[10px] text-amber-400">
                          Reverses {tx.reversal_of}
                        </span>
                      )}
                    </td>
                    <td className="py-3 px-4 space-y-1">
                      {tx.postings.map((p, i) => {
                        const acc = accounts.find((a) => a.id === p.account_id);
                        return (
                          <div key={i} className="text-[11px] flex items-center space-x-2">
                            <span
                              className={`px-1.5 py-0.2 rounded font-mono text-[9px] ${
                                p.direction === 'Debit'
                                  ? 'bg-blue-500/10 text-blue-400'
                                  : 'bg-emerald-500/10 text-emerald-400'
                              }`}
                            >
                              {p.direction}
                            </span>
                            <span className="text-slate-300 font-medium">
                              {acc?.name || p.account_id}
                            </span>
                            <span className="text-slate-400 font-mono">
                              {formatINR(p.amount_paisa)}
                            </span>
                          </div>
                        );
                      })}
                    </td>
                    <td className="py-3 px-4 text-right font-mono font-bold text-white whitespace-nowrap">
                      {formatINR(totalAmount)}
                    </td>
                    <td className="py-3 px-4 text-center">
                      {!isReversal && !tx.reversal_of && (
                        <button
                          onClick={() => setReversalModalTx(tx)}
                          title="Reverse Transaction"
                          className="p-1.5 rounded hover:bg-slate-700/60 text-slate-400 hover:text-amber-400 transition"
                        >
                          <RotateCcw className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal: New Double-Entry Transaction */}
      {txModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <div className="bg-[#111726] border border-slate-800 rounded-xl max-w-2xl w-full p-6 space-y-6 shadow-2xl">
            <div className="flex items-center justify-between border-b border-slate-800 pb-4">
              <div>
                <h3 className="text-lg font-bold text-white">Post Double-Entry Transaction</h3>
                <p className="text-xs text-slate-400">
                  Total Debits must mathematically equal Total Credits
                </p>
              </div>
              <button
                onClick={() => setTxModalOpen(false)}
                className="text-slate-400 hover:text-white"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleSubmitTx} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">
                    Transaction Type
                  </label>
                  <select
                    value={txType}
                    onChange={(e) => setTxType(e.target.value as TransactionType)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 focus:outline-none"
                  >
                    <option value="Expense">Expense</option>
                    <option value="Income">Income</option>
                    <option value="Transfer">Transfer</option>
                    <option value="Deposit">Deposit</option>
                    <option value="Withdrawal">Withdrawal</option>
                    <option value="Adjustment">Adjustment</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">
                    Category
                  </label>
                  <input
                    type="text"
                    placeholder="e.g. Rent, Groceries, Salary"
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 focus:outline-none"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">
                    Reference / UTR
                  </label>
                  <input
                    type="text"
                    placeholder="e.g. UPI / CMS ID"
                    value={refNumber}
                    onChange={(e) => setRefNumber(e.target.value)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 focus:outline-none"
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">
                    Memo Notes
                  </label>
                  <input
                    type="text"
                    placeholder="Optional memo"
                    value={notes}
                    onChange={(e) => setNotes(e.target.value)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200 focus:outline-none"
                  />
                </div>
              </div>

              {/* Postings Legs */}
              <div className="border border-slate-800 rounded-lg p-4 bg-slate-900/40 space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-white uppercase tracking-wider">
                    Posting Legs
                  </span>
                  <button
                    type="button"
                    onClick={handleAddPosting}
                    className="text-xs text-emerald-400 hover:text-emerald-300"
                  >
                    + Add Leg
                  </button>
                </div>

                {postings.map((p, idx) => (
                  <div key={idx} className="grid grid-cols-12 gap-2 items-center">
                    <select
                      value={p.account_id}
                      onChange={(e) => handlePostingChange(idx, 'account_id', e.target.value)}
                      className="col-span-6 bg-slate-900 border border-slate-700 text-xs rounded-md p-2 text-slate-200"
                      required
                    >
                      <option value="">Select Account...</option>
                      {accounts.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.name} ({a.account_type})
                        </option>
                      ))}
                    </select>

                    <select
                      value={p.direction}
                      onChange={(e) => handlePostingChange(idx, 'direction', e.target.value)}
                      className="col-span-3 bg-slate-900 border border-slate-700 text-xs rounded-md p-2 text-slate-200 font-mono"
                    >
                      <option value="Debit">Debit</option>
                      <option value="Credit">Credit</option>
                    </select>

                    <input
                      type="number"
                      step="0.01"
                      min="0.01"
                      placeholder="₹ Amount"
                      value={p.rupees}
                      onChange={(e) => handlePostingChange(idx, 'rupees', e.target.value)}
                      className="col-span-3 bg-slate-900 border border-slate-700 text-xs rounded-md p-2 text-slate-200 font-mono text-right"
                      required
                    />
                  </div>
                ))}

                {/* Equilibrium Verifier Pill */}
                <div className="pt-3 border-t border-slate-800 flex items-center justify-between text-xs">
                  <div className="flex items-center space-x-3">
                    <span className="text-slate-400">
                      Debits: <span className="text-white font-mono">{formatINR(totalDebits)}</span>
                    </span>
                    <span className="text-slate-400">
                      Credits: <span className="text-white font-mono">{formatINR(totalCredits)}</span>
                    </span>
                  </div>

                  <div>
                    {isBalanced ? (
                      <span className="flex items-center space-x-1 text-emerald-400 font-semibold">
                        <CheckCircle2 className="w-3.5 h-3.5" />
                        <span>Equilibrium: Balanced</span>
                      </span>
                    ) : (
                      <span className="text-rose-400 font-semibold">
                        Unbalanced Diff: {formatINR(Math.abs(totalDebits - totalCredits))}
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <div className="flex items-center justify-end space-x-3 pt-4 border-t border-slate-800">
                <button
                  type="button"
                  onClick={() => setTxModalOpen(false)}
                  className="px-4 py-2 rounded-lg bg-slate-800 text-slate-300 text-xs font-semibold hover:bg-slate-700"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!isBalanced}
                  className="px-5 py-2 rounded-lg bg-emerald-600 disabled:opacity-40 text-white text-xs font-semibold hover:bg-emerald-500 shadow-sm transition"
                >
                  Post Balanced Entry
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Modal: Reversal Confirmation */}
      {reversalModalTx && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <div className="bg-[#111726] border border-slate-800 rounded-xl max-w-md w-full p-6 space-y-5">
            <div className="flex items-center space-x-3 text-amber-400">
              <RotateCcw className="w-6 h-6" />
              <h3 className="text-base font-bold text-white">Reverse Transaction</h3>
            </div>
            <p className="text-xs text-slate-300">
              This creates a reciprocal balanced entry with inverted Debits and Credits. Original
              entry will remain permanently recorded in the audit trail.
            </p>
            <div>
              <label className="block text-xs font-semibold text-slate-300 mb-1">
                Reason for Reversal
              </label>
              <input
                type="text"
                placeholder="e.g. Erroneous double posting"
                value={reversalReason}
                onChange={(e) => setReversalReason(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
              />
            </div>
            <div className="flex justify-end space-x-3">
              <button
                onClick={() => setReversalModalTx(null)}
                className="px-4 py-2 bg-slate-800 text-xs font-semibold text-slate-300 rounded-lg hover:bg-slate-700"
              >
                Cancel
              </button>
              <button
                onClick={handleExecuteReversal}
                disabled={!reversalReason.trim()}
                className="px-4 py-2 bg-amber-600 hover:bg-amber-500 disabled:opacity-40 text-xs font-semibold text-white rounded-lg shadow-sm"
              >
                Confirm Reversal
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: CSV Export Security Warning */}
      {csvWarningOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <div className="bg-[#111726] border border-rose-500/40 rounded-xl max-w-md w-full p-6 space-y-4">
            <div className="flex items-center space-x-3 text-rose-400">
              <AlertOctagon className="w-6 h-6" />
              <h3 className="text-base font-bold text-white">Security Warning: Plaintext Export</h3>
            </div>
            <p className="text-xs text-slate-300 leading-relaxed">
              Exported CSV files are saved in <strong>unencrypted plaintext</strong> on your physical disk.
              Anyone with access to the export location can read your account balances and transaction history.
            </p>
            <p className="text-xs text-slate-400">
              Only export if you are copying to an encrypted offline volume, and securely shred the file when done.
            </p>
            <div className="flex justify-end space-x-3 pt-2">
              <button
                onClick={() => setCsvWarningOpen(false)}
                className="px-4 py-2 bg-slate-800 text-xs font-semibold text-slate-300 rounded-lg"
              >
                Cancel
              </button>
              <button
                onClick={handleExportCsv}
                className="px-4 py-2 bg-rose-600 hover:bg-rose-500 text-xs font-semibold text-white rounded-lg shadow-sm"
              >
                Acknowledge & Export
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
