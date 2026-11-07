import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Account, Direction, Transaction, TransactionType } from '../types';
import { formatINR, formatDate } from '../utils/format';
import {
  Plus,
  RotateCcw,
  Download,
  Search,
  CheckCircle2,
  AlertTriangle,
  X,
  Trash2,
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
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [loading, setLoading] = useState(true);

  // New Transaction Form State
  const [txType, setTxType] = useState<TransactionType>('Expense');
  const [category, setCategory] = useState('');
  const [refNumber, setRefNumber] = useState('');
  const [notes, setNotes] = useState('');
  const [paymentMethod, setPaymentMethod] = useState('NetBanking');
  const [postings, setPostings] = useState<
    Array<{ account_id: string; direction: Direction; rupees: string; memo: string }>
  >([
    { account_id: '', direction: 'Debit', rupees: '', memo: '' },
    { account_id: '', direction: 'Credit', rupees: '', memo: '' },
  ]);

  const loadData = async () => {
    try {
      const txs = await api.getTransactions(200, 0);
      const accs = await api.getAccounts();
      setTransactions(txs);
      setAccounts(accs);
    } catch (err) {
      console.error('Failed to load transactions:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  // Postings balance calculation in paisa
  const totalDebits = postings
    .filter((p) => p.direction === 'Debit')
    .reduce((sum, p) => sum + Math.round((parseFloat(p.rupees) || 0) * 100), 0);

  const totalCredits = postings
    .filter((p) => p.direction === 'Credit')
    .reduce((sum, p) => sum + Math.round((parseFloat(p.rupees) || 0) * 100), 0);

  const difference = totalDebits - totalCredits;
  const isBalanced = totalDebits > 0 && difference === 0;

  const handleAddPosting = () => {
    setPostings([...postings, { account_id: '', direction: 'Credit', rupees: '', memo: '' }]);
  };

  const handleRemovePosting = (idx: number) => {
    if (postings.length <= 2) return;
    setPostings(postings.filter((_, i) => i !== idx));
  };

  const handlePostingChange = (index: number, field: string, val: string) => {
    const updated = [...postings];
    updated[index] = { ...updated[index], [field]: val };
    setPostings(updated);
  };

  const handleSubmitTx = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!isBalanced) {
      alert('Transaction must be strictly balanced before posting (Debits must equal Credits).');
      return;
    }

    setIsSubmitting(true);
    try {
      const validPostings = postings.map((p) => ({
        account_id: p.account_id,
        direction: p.direction,
        amount_paisa: Math.round(parseFloat(p.rupees) * 100),
        memo: p.memo.trim() || undefined,
      }));

      await api.postTransaction({
        tx_type: txType,
        postings: validPostings,
        category: category.trim() || undefined,
        reference: refNumber.trim() || undefined,
        notes: notes.trim() || undefined,
        payment_method: paymentMethod,
      });

      setTxModalOpen(false);
      setPostings([
        { account_id: '', direction: 'Debit', rupees: '', memo: '' },
        { account_id: '', direction: 'Credit', rupees: '', memo: '' },
      ]);
      setCategory('');
      setRefNumber('');
      setNotes('');
      await loadData();
    } catch (err: any) {
      alert(`Error posting transaction: ${err.message}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleExecuteReversal = async () => {
    if (!reversalModalTx || !reversalReason.trim()) return;
    setIsSubmitting(true);
    try {
      await api.reverseTransaction(reversalModalTx.id, reversalReason.trim());
      setReversalModalTx(null);
      setReversalReason('');
      await loadData();
    } catch (err: any) {
      alert(`Reversal failed: ${err.message}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleExportCsv = () => {
    let csv =
      '# WARNING: UNENCRYPTED FINANCIAL RECORD EXPORTED FROM BHOOT OS\nTransactionID,Date,Type,Category,Debit_Account,Credit_Account,Amount_INR,Status\n';
    transactions.forEach((t) => {
      const dAcc = accounts.find((a) => a.id === t.postings[0]?.account_id)?.name || '';
      const cAcc = accounts.find((a) => a.id === t.postings[1]?.account_id)?.name || '';
      const amt = (t.postings[0]?.amount_paisa || 0) / 100;
      csv += `"${t.id}","${t.timestamp}","${t.tx_type}","${t.category || ''}","${dAcc}","${cAcc}",${amt},"${t.verification_status}"\n`;
    });
    api.exportCsv(csv, `bhoot_transactions_${Date.now()}.csv`);
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

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Loading Transactions Ledger...
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-6 max-w-7xl mx-auto">
      {/* View Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Ledger Transactions</h1>
          <p className="text-xs text-neutral-400">
            Strict double-entry journal entries with cryptographic chain integrity
          </p>
        </div>

        <div className="flex items-center space-x-3">
          <button
            onClick={handleExportCsv}
            className="flex items-center space-x-1.5 px-3 py-2 bg-[#181818] hover:bg-[#222222] text-white text-xs font-semibold rounded-lg border border-[#333333] transition"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Export CSV</span>
          </button>

          <button
            onClick={() => setTxModalOpen(true)}
            className="flex items-center space-x-1.5 px-4 py-2 bg-white hover:bg-neutral-200 text-black text-xs font-semibold rounded-lg shadow-sm transition"
          >
            <Plus className="w-4 h-4" />
            <span>New Transaction</span>
          </button>
        </div>
      </div>

      {/* Filter & Search Bar */}
      <div className="flex items-center space-x-3 bg-[#111111] p-3 rounded-lg border border-[#222222]">
        <Search className="w-4 h-4 text-neutral-500" />
        <input
          type="text"
          placeholder="Search by category, memo, reference ID, or transaction type..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="bg-transparent text-xs text-white placeholder-neutral-500 focus:outline-none flex-1 font-mono"
        />
      </div>

      {/* Transactions Table */}
      <div className="bg-[#111111] rounded-xl border border-[#222222] overflow-hidden shadow-sm">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-[#222222] bg-[#141414] text-neutral-400 font-semibold uppercase tracking-wider">
                <th className="py-3 px-4">Date</th>
                <th className="py-3 px-4">Type</th>
                <th className="py-3 px-4">Category</th>
                <th className="py-3 px-4">Reference / Notes</th>
                <th className="py-3 px-4">Debits & Credits (Legs)</th>
                <th className="py-3 px-4 text-right">Balanced Total</th>
                <th className="py-3 px-4 text-center">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#222222]/80">
              {filteredTxs.map((tx) => {
                const totalAmount = tx.postings[0]?.amount_paisa ?? 0;
                const isReversal = tx.tx_type === 'Reversal';

                return (
                  <tr key={tx.id} className="hover:bg-[#161616] transition">
                    <td className="py-3 px-4 text-neutral-300 font-mono whitespace-nowrap">
                      {formatDate(tx.timestamp)}
                    </td>
                    <td className="py-3 px-4">
                      <span
                        className={`px-2 py-0.5 rounded text-[10px] font-semibold border ${
                          isReversal
                            ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                            : 'bg-[#181818] text-white border-[#333333]'
                        }`}
                      >
                        {tx.tx_type}
                      </span>
                    </td>
                    <td className="py-3 px-4 text-neutral-300">{tx.category || '—'}</td>
                    <td className="py-3 px-4 text-neutral-400 font-mono text-[11px]">
                      {tx.reference_number || tx.notes || '—'}
                      {tx.reversal_of && (
                        <span className="block text-[10px] text-amber-400">
                          Reverses {tx.reversal_of.slice(0, 8)}...
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
                                  ? 'bg-neutral-800 text-neutral-200 border border-neutral-700'
                                  : 'bg-neutral-900 text-neutral-400 border border-neutral-800'
                              }`}
                            >
                              {p.direction}
                            </span>
                            <span className="text-neutral-300 font-medium">
                              {acc?.name || p.account_id.slice(0, 8)}
                            </span>
                            <span className="text-neutral-400 font-mono">
                              {formatINR(p.amount_paisa)}
                            </span>
                            {p.memo && <span className="text-neutral-500 italic text-[10px]">({p.memo})</span>}
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
                          className="px-2 py-1 rounded text-[11px] font-medium bg-[#181818] hover:bg-[#222222] text-neutral-300 hover:text-white border border-[#333333] transition flex items-center space-x-1 mx-auto"
                        >
                          <RotateCcw className="w-3 h-3" />
                          <span>Reverse</span>
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
              {filteredTxs.length === 0 && (
                <tr>
                  <td colSpan={7} className="py-8 text-center text-xs text-neutral-500">
                    No matching transactions found.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal: New Balanced Transaction */}
      {txModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4 overflow-y-auto">
          <div className="bg-[#111111] border border-[#222222] rounded-xl max-w-2xl w-full p-6 space-y-5 my-8 shadow-2xl">
            <div className="flex items-center justify-between border-b border-[#222222] pb-3">
              <div>
                <h3 className="text-base font-bold text-white">Post Double-Entry Transaction</h3>
                <p className="text-xs text-neutral-400">Strictly balanced double-entry invariant enforced</p>
              </div>
              <button onClick={() => setTxModalOpen(false)} className="text-neutral-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleSubmitTx} className="space-y-4">
              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Type</label>
                  <select
                    value={txType}
                    onChange={(e) => setTxType(e.target.value as TransactionType)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  >
                    <option value="Expense">Expense</option>
                    <option value="Income">Income</option>
                    <option value="Transfer">Transfer</option>
                    <option value="Adjustment">Adjustment</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Category</label>
                  <input
                    type="text"
                    placeholder="e.g. Groceries, Rent, Salary"
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Payment Method</label>
                  <select
                    value={paymentMethod}
                    onChange={(e) => setPaymentMethod(e.target.value)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  >
                    <option value="NetBanking">Net Banking</option>
                    <option value="UPI">UPI</option>
                    <option value="CreditCard">Credit Card</option>
                    <option value="DebitCard">Debit Card</option>
                    <option value="Cash">Cash</option>
                    <option value="Cheque">Cheque</option>
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Reference Number</label>
                  <input
                    type="text"
                    placeholder="e.g. UPI-928194, CHQ-10492"
                    value={refNumber}
                    onChange={(e) => setRefNumber(e.target.value)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none font-mono"
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Notes / Description</label>
                  <input
                    type="text"
                    placeholder="Detailed entry context"
                    value={notes}
                    onChange={(e) => setNotes(e.target.value)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  />
                </div>
              </div>

              {/* Multi-Leg Postings Table */}
              <div className="space-y-2 pt-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-white uppercase tracking-wider">
                    Transaction Legs (Postings)
                  </span>
                  <button
                    type="button"
                    onClick={handleAddPosting}
                    className="text-xs text-white hover:text-neutral-300 flex items-center space-x-1 font-semibold"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>Add Leg</span>
                  </button>
                </div>

                <div className="space-y-2">
                  {postings.map((p, idx) => (
                    <div key={idx} className="flex items-center space-x-2 bg-[#181818] p-2.5 rounded-lg border border-[#222222]">
                      <select
                        value={p.account_id}
                        onChange={(e) => handlePostingChange(idx, 'account_id', e.target.value)}
                        className="flex-1 bg-[#111111] border border-[#333333] text-xs rounded p-2 text-white focus:border-white focus:outline-none"
                        required
                      >
                        <option value="">Select Account</option>
                        {accounts.map((a) => (
                          <option key={a.id} value={a.id}>
                            {a.name} ({a.account_type})
                          </option>
                        ))}
                      </select>

                      <select
                        value={p.direction}
                        onChange={(e) => handlePostingChange(idx, 'direction', e.target.value)}
                        className="w-24 bg-[#111111] border border-[#333333] text-xs rounded p-2 text-white focus:border-white focus:outline-none"
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
                        className="w-28 bg-[#111111] border border-[#333333] text-xs rounded p-2 text-white font-mono focus:border-white focus:outline-none"
                        required
                      />

                      <input
                        type="text"
                        placeholder="Memo (opt)"
                        value={p.memo}
                        onChange={(e) => handlePostingChange(idx, 'memo', e.target.value)}
                        className="w-32 bg-[#111111] border border-[#333333] text-xs rounded p-2 text-white focus:border-white focus:outline-none text-[11px]"
                      />

                      {postings.length > 2 && (
                        <button
                          type="button"
                          onClick={() => handleRemovePosting(idx)}
                          className="p-1.5 text-neutral-500 hover:text-red-400 rounded transition"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  ))}
                </div>

                {/* Equilibrium Invariant Status Bar */}
                <div
                  className={`p-3 rounded-lg border flex items-center justify-between text-xs font-mono transition ${
                    isBalanced
                      ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
                      : 'bg-red-500/10 border-red-500/30 text-red-400'
                  }`}
                >
                  <div className="flex items-center space-x-2">
                    {isBalanced ? (
                      <CheckCircle2 className="w-4 h-4" />
                    ) : (
                      <AlertTriangle className="w-4 h-4" />
                    )}
                    <span>
                      {isBalanced
                        ? 'Equilibrium Invariant Satisfied: Σ Debits = Σ Credits'
                        : `Unbalanced: Difference = ₹${(Math.abs(difference) / 100).toFixed(2)}`}
                    </span>
                  </div>
                  <div>
                    Debits: ₹{(totalDebits / 100).toFixed(2)} | Credits: ₹{(totalCredits / 100).toFixed(2)}
                  </div>
                </div>
              </div>

              <div className="flex justify-end space-x-3 pt-3 border-t border-[#222222]">
                <button
                  type="button"
                  onClick={() => setTxModalOpen(false)}
                  className="px-4 py-2 bg-[#181818] border border-[#333333] text-white text-xs font-semibold rounded-lg hover:bg-[#222222] transition"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!isBalanced || isSubmitting}
                  className="px-4 py-2 bg-white hover:bg-neutral-200 disabled:opacity-40 text-black text-xs font-semibold rounded-lg shadow-sm transition"
                >
                  {isSubmitting ? 'Posting Entry...' : 'Post Transaction'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Modal: Reversal Confirmation */}
      {reversalModalTx && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4">
          <div className="bg-[#111111] border border-[#222222] rounded-xl max-w-md w-full p-6 space-y-4 shadow-2xl">
            <div className="flex items-center justify-between border-b border-[#222222] pb-3">
              <h3 className="text-base font-bold text-white">Reverse Transaction</h3>
              <button onClick={() => setReversalModalTx(null)} className="text-neutral-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <p className="text-xs text-neutral-400 leading-relaxed">
              Accounting records are append-only. Reversing this transaction will create an inverse entry
              referencing transaction <span className="text-white font-mono">{reversalModalTx.id.slice(0, 8)}...</span>
            </p>

            <div>
              <label className="block text-xs font-semibold text-neutral-300 mb-1">
                Reason for Reversal
              </label>
              <input
                type="text"
                placeholder="e.g. Inadvertent duplicate entry, incorrect amount"
                value={reversalReason}
                onChange={(e) => setReversalReason(e.target.value)}
                className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                required
              />
            </div>

            <div className="flex justify-end space-x-3 pt-3 border-t border-[#222222]">
              <button
                type="button"
                onClick={() => setReversalModalTx(null)}
                className="px-4 py-2 bg-[#181818] border border-[#333333] text-white text-xs font-semibold rounded-lg hover:bg-[#222222] transition"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleExecuteReversal}
                disabled={!reversalReason.trim() || isSubmitting}
                className="px-4 py-2 bg-white hover:bg-neutral-200 disabled:opacity-50 text-black text-xs font-semibold rounded-lg shadow-sm transition"
              >
                {isSubmitting ? 'Reversing...' : 'Confirm Reversal'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
