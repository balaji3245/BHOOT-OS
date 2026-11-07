import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Account, AccountSubtype, AccountType } from '../types';
import { formatINR } from '../utils/format';
import { Plus, X, Building2, CreditCard, PiggyBank, Briefcase, Archive } from 'lucide-react';

export const AccountsView: React.FC = () => {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [balances, setBalances] = useState<Record<string, number>>({});
  const [modalOpen, setModalOpen] = useState(false);
  const [loading, setLoading] = useState(true);

  // New Account Form
  const [name, setName] = useState('');
  const [accountType, setAccountType] = useState<AccountType>('Asset');
  const [subtype, setSubtype] = useState<AccountSubtype>('BankAccount');
  const [description, setDescription] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const loadData = async () => {
    try {
      const accs = await api.getAccounts();
      setAccounts(accs);
      const bals: Record<string, number> = {};
      for (const a of accs) {
        bals[a.id] = await api.getAccountBalance(a.id);
      }
      setBalances(bals);
    } catch (err) {
      console.error('Failed to load accounts:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleCreateAccount = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setIsSubmitting(true);
    try {
      await api.createAccount({
        name: name.trim(),
        account_type: accountType,
        subtype,
        currency: 'INR',
        description: description.trim() || undefined,
      });
      setModalOpen(false);
      setName('');
      setDescription('');
      await loadData();
    } catch (err: any) {
      alert(`Failed to create account: ${err.message}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleToggleStatus = async (account: Account) => {
    const nextStatus = account.status === 'Active' ? 'Archived' : 'Active';
    try {
      await api.updateAccountStatus(account.id, nextStatus as 'Active' | 'Archived');
      await loadData();
    } catch (err: any) {
      alert(`Failed to update status: ${err.message}`);
    }
  };

  const getSubtypeIcon = (st: AccountSubtype) => {
    switch (st) {
      case 'BankAccount':
      case 'UPIWallet':
        return <Building2 className="w-4 h-4 text-white" />;
      case 'CreditCard':
      case 'PersonalLoan':
        return <CreditCard className="w-4 h-4 text-white" />;
      case 'BrokerAccount':
      case 'InvestmentAccount':
        return <Briefcase className="w-4 h-4 text-white" />;
      default:
        return <PiggyBank className="w-4 h-4 text-white" />;
    }
  };

  const types: AccountType[] = ['Asset', 'Liability', 'Equity', 'Income', 'Expense'];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Loading Chart of Accounts...
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Chart of Accounts</h1>
          <p className="text-xs text-neutral-400">
            Categorized double-entry ledger accounts enforcing normal balance conventions
          </p>
        </div>
        <button
          onClick={() => setModalOpen(true)}
          className="flex items-center space-x-1.5 px-4 py-2 bg-white hover:bg-neutral-200 text-black text-xs font-semibold rounded-lg shadow-sm transition"
        >
          <Plus className="w-4 h-4" />
          <span>Add Account</span>
        </button>
      </div>

      <div className="space-y-6">
        {types.map((t) => {
          const accs = accounts.filter((a) => a.account_type === t);
          if (accs.length === 0) return null;

          const totalBalance = accs.reduce((sum, a) => sum + (balances[a.id] || 0), 0);

          return (
            <div key={t} className="bg-[#111111] rounded-xl border border-[#222222] p-5 space-y-4">
              <div className="flex items-center justify-between border-b border-[#222222] pb-3">
                <div className="flex items-center space-x-2">
                  <span className="text-sm font-bold text-white uppercase tracking-wider">{t} Accounts</span>
                  <span className="text-xs text-neutral-500">({accs.length})</span>
                </div>
                <div className="text-xs font-mono font-bold text-white">
                  Total: {formatINR(totalBalance)}
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {accs.map((a) => (
                  <div
                    key={a.id}
                    className="p-4 rounded-lg bg-[#181818] border border-[#222222] flex flex-col justify-between space-y-3"
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex items-center space-x-2.5">
                        <div className="p-2 rounded bg-[#111111] border border-[#333333]">
                          {getSubtypeIcon(a.subtype)}
                        </div>
                        <div>
                          <h4 className="text-sm font-bold text-white">{a.name}</h4>
                          <span className="text-[10px] text-neutral-400 font-mono">{a.subtype}</span>
                        </div>
                      </div>
                      <div className="flex items-center space-x-1.5">
                        <span
                          className={`text-[9px] px-1.5 py-0.5 rounded font-mono ${
                            a.status === 'Active'
                              ? 'bg-neutral-800 text-neutral-300 border border-neutral-700'
                              : 'bg-amber-500/10 text-amber-400 border border-amber-500/30'
                          }`}
                        >
                          {a.status}
                        </span>
                        <button
                          onClick={() => handleToggleStatus(a)}
                          title={a.status === 'Active' ? 'Archive account' : 'Reactivate account'}
                          className="p-1 text-neutral-500 hover:text-white rounded transition"
                        >
                          <Archive className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>

                    {a.description && (
                      <p className="text-xs text-neutral-400 line-clamp-1">{a.description}</p>
                    )}

                    <div className="pt-2 border-t border-[#222222] flex items-center justify-between">
                      <span className="text-xs text-neutral-400">Current Balance</span>
                      <span className="text-sm font-bold font-mono text-white">
                        {formatINR(balances[a.id] || 0)}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>

      {/* Modal: New Account */}
      {modalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4">
          <div className="bg-[#111111] border border-[#222222] rounded-xl max-w-md w-full p-6 space-y-4 shadow-2xl">
            <div className="flex items-center justify-between border-b border-[#222222] pb-3">
              <h3 className="text-base font-bold text-white">Create Ledger Account</h3>
              <button onClick={() => setModalOpen(false)} className="text-neutral-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleCreateAccount} className="space-y-4">
              <div>
                <label className="block text-xs font-semibold text-neutral-300 mb-1">Account Name</label>
                <input
                  type="text"
                  placeholder="e.g. Axis Salary Bank, Zerodha Demat"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  required
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Account Type</label>
                  <select
                    value={accountType}
                    onChange={(e) => setAccountType(e.target.value as AccountType)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  >
                    <option value="Asset">Asset</option>
                    <option value="Liability">Liability</option>
                    <option value="Equity">Equity</option>
                    <option value="Income">Income</option>
                    <option value="Expense">Expense</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-semibold text-neutral-300 mb-1">Subtype</label>
                  <select
                    value={subtype}
                    onChange={(e) => setSubtype(e.target.value as AccountSubtype)}
                    className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  >
                    <option value="BankAccount">Bank Account</option>
                    <option value="UPIWallet">UPI Wallet</option>
                    <option value="CashWallet">Cash Wallet</option>
                    <option value="BrokerAccount">Broker / Demat</option>
                    <option value="CreditCard">Credit Card</option>
                    <option value="PersonalLoan">Loan</option>
                    <option value="Salary">Salary Revenue</option>
                    <option value="LivingExpense">Living Expense</option>
                  </select>
                </div>
              </div>

              <div>
                <label className="block text-xs font-semibold text-neutral-300 mb-1">Description</label>
                <input
                  type="text"
                  placeholder="Optional notes or account number"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                />
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
                  {isSubmitting ? 'Creating...' : 'Create Account'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
