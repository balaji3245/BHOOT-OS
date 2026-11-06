import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Account, AccountSubtype, AccountType } from '../types';
import { formatINR } from '../utils/format';
import { Plus, X, Building2, CreditCard, PiggyBank, Briefcase } from 'lucide-react';

export const AccountsView: React.FC = () => {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [balances, setBalances] = useState<Record<string, number>>({});
  const [modalOpen, setModalOpen] = useState(false);

  // New Account Form
  const [name, setName] = useState('');
  const [accountType, setAccountType] = useState<AccountType>('Asset');
  const [subtype, setSubtype] = useState<AccountSubtype>('BankAccount');
  const [description, setDescription] = useState('');

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
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleCreateAccount = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    try {
      await api.createAccount({
        name: name.trim(),
        account_type: accountType,
        subtype,
        status: 'Active',
        currency: 'INR',
        description: description.trim() || undefined,
      });
      setModalOpen(false);
      setName('');
      setDescription('');
      loadData();
    } catch (err: any) {
      alert(`Failed to create account: ${err.message}`);
    }
  };

  const getSubtypeIcon = (st: AccountSubtype) => {
    switch (st) {
      case 'BankAccount':
      case 'UPIWallet':
        return <Building2 className="w-4 h-4 text-emerald-400" />;
      case 'CreditCard':
      case 'PersonalLoan':
        return <CreditCard className="w-4 h-4 text-rose-400" />;
      case 'BrokerAccount':
      case 'InvestmentAccount':
        return <Briefcase className="w-4 h-4 text-blue-400" />;
      default:
        return <PiggyBank className="w-4 h-4 text-amber-400" />;
    }
  };

  const types: AccountType[] = ['Asset', 'Liability', 'Equity', 'Income', 'Expense'];

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Chart of Accounts</h1>
          <p className="text-xs text-slate-400">
            Categorized double-entry ledger accounts enforcing normal balance conventions
          </p>
        </div>
        <button
          onClick={() => setModalOpen(true)}
          className="flex items-center space-x-1.5 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition"
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
            <div key={t} className="bg-[#111726] rounded-xl border border-slate-800 p-5 space-y-4">
              <div className="flex items-center justify-between border-b border-slate-800 pb-3">
                <div className="flex items-center space-x-2">
                  <span className="text-sm font-bold text-white uppercase tracking-wider">{t} Accounts</span>
                  <span className="text-xs text-slate-500">({accs.length})</span>
                </div>
                <div className="text-xs font-mono font-bold text-emerald-400">
                  Total: {formatINR(totalBalance)}
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {accs.map((a) => (
                  <div
                    key={a.id}
                    className="p-4 rounded-lg bg-slate-900/60 border border-slate-800 flex flex-col justify-between space-y-3"
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex items-center space-x-2.5">
                        <div className="p-2 rounded bg-slate-800 border border-slate-700">
                          {getSubtypeIcon(a.subtype)}
                        </div>
                        <div>
                          <h4 className="text-sm font-bold text-white">{a.name}</h4>
                          <span className="text-[10px] text-slate-400 font-mono">{a.subtype}</span>
                        </div>
                      </div>
                      <span className="text-[10px] bg-slate-800 text-slate-300 px-1.5 py-0.5 rounded font-mono">
                        {a.currency}
                      </span>
                    </div>

                    {a.description && (
                      <p className="text-xs text-slate-400 line-clamp-1">{a.description}</p>
                    )}

                    <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between">
                      <span className="text-xs text-slate-400">Current Balance</span>
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
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <div className="bg-[#111726] border border-slate-800 rounded-xl max-w-md w-full p-6 space-y-4">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-white">Create Ledger Account</h3>
              <button onClick={() => setModalOpen(false)} className="text-slate-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleCreateAccount} className="space-y-4">
              <div>
                <label className="block text-xs font-semibold text-slate-300 mb-1">Account Name</label>
                <input
                  type="text"
                  placeholder="e.g. Axis Salary Bank"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                  required
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">Account Type</label>
                  <select
                    value={accountType}
                    onChange={(e) => setAccountType(e.target.value as AccountType)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                  >
                    <option value="Asset">Asset</option>
                    <option value="Liability">Liability</option>
                    <option value="Equity">Equity</option>
                    <option value="Income">Income</option>
                    <option value="Expense">Expense</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">Subtype</label>
                  <select
                    value={subtype}
                    onChange={(e) => setSubtype(e.target.value as AccountSubtype)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
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
                <label className="block text-xs font-semibold text-slate-300 mb-1">Description</label>
                <input
                  type="text"
                  placeholder="Optional notes or account number"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                />
              </div>

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
                  Create Account
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
