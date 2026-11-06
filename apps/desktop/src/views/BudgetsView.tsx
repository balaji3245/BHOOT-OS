import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Budget, BudgetPeriod, BudgetReport } from '../types';
import { formatINR, formatBps } from '../utils/format';
import { Plus, AlertTriangle, CheckCircle, Flame, X } from 'lucide-react';

export const BudgetsView: React.FC = () => {
  const [budgets, setBudgets] = useState<Budget[]>([]);
  const [activeBudgetReport, setActiveBudgetReport] = useState<BudgetReport | null>(null);
  const [modalOpen, setModalOpen] = useState(false);

  // New Budget Form
  const [budgetName, setBudgetName] = useState('');
  const [period, setPeriod] = useState<BudgetPeriod>('Monthly');
  const [categories, setCategories] = useState<
    Array<{ category: string; allocatedRupees: string; threshold: number }>
  >([
    { category: 'Rent', allocatedRupees: '40000', threshold: 90 },
    { category: 'Groceries', allocatedRupees: '20000', threshold: 75 },
    { category: 'Utilities', allocatedRupees: '10000', threshold: 80 },
  ]);

  const loadData = async () => {
    try {
      const bgts = await api.getBudgets();
      setBudgets(bgts);
      if (bgts.length > 0) {
        const rep = await api.evaluateBudget(bgts[0].id);
        setActiveBudgetReport(rep);
      }
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleSelectBudget = async (id: string) => {
    const rep = await api.evaluateBudget(id);
    setActiveBudgetReport(rep);
  };

  const handleAddCategoryRow = () => {
    setCategories([...categories, { category: '', allocatedRupees: '', threshold: 80 }]);
  };

  const handleCategoryChange = (idx: number, field: string, val: any) => {
    const updated = [...categories];
    updated[idx] = { ...updated[idx], [field]: val };
    setCategories(updated);
  };

  const handleCreateBudget = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!budgetName.trim()) return;

    try {
      const now = new Date();
      const end = new Date(now.getTime() + 30 * 86400000);

      const cats = categories.map((c) => ({
        category: c.category.trim(),
        allocated_paisa: Math.round(parseFloat(c.allocatedRupees || '0') * 100),
        alert_threshold_pct: c.threshold,
      }));

      await api.saveBudget({
        name: budgetName.trim(),
        period,
        start_date: now.toISOString(),
        end_date: end.toISOString(),
        categories: cats,
      });

      setModalOpen(false);
      setBudgetName('');
      loadData();
    } catch (err: any) {
      alert(`Failed to save budget: ${err.message}`);
    }
  };

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Budgets & Limits</h1>
          <p className="text-xs text-slate-400">
            Periodic spending thresholds with automated early warning alerts
          </p>
        </div>
        <button
          onClick={() => setModalOpen(true)}
          className="flex items-center space-x-1.5 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition"
        >
          <Plus className="w-4 h-4" />
          <span>New Budget Plan</span>
        </button>
      </div>

      {/* Active Budget Selector */}
      {budgets.length > 1 && (
        <div className="flex items-center space-x-2">
          {budgets.map((b) => (
            <button
              key={b.id}
              onClick={() => handleSelectBudget(b.id)}
              className={`px-3 py-1.5 rounded-lg text-xs font-semibold border transition ${
                activeBudgetReport?.budget_id === b.id
                  ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/40'
                  : 'bg-slate-800 text-slate-400 border-slate-700'
              }`}
            >
              {b.name}
            </button>
          ))}
        </div>
      )}

      {/* Main Budget Evaluation Dashboard */}
      {activeBudgetReport ? (
        <div className="space-y-6">
          {/* Top Progress Overview Card */}
          <div className="p-6 rounded-xl bg-[#111726] border border-slate-800 space-y-4">
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
              <div>
                <h3 className="text-lg font-bold text-white">{activeBudgetReport.budget_name}</h3>
                <p className="text-xs text-slate-400">
                  Plan Period: <span className="text-slate-200">{activeBudgetReport.period}</span>
                </p>
              </div>
              <div className="flex items-center space-x-6 text-xs font-mono">
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase">Allocated</span>
                  <span className="text-sm font-bold text-white">
                    {formatINR(activeBudgetReport.total_allocated_paisa)}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase">Spent</span>
                  <span className="text-sm font-bold text-emerald-400">
                    {formatINR(activeBudgetReport.total_spent_paisa)}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase">Remaining</span>
                  <span className="text-sm font-bold text-white">
                    {formatINR(activeBudgetReport.total_remaining_paisa)}
                  </span>
                </div>
              </div>
            </div>

            {/* Overall Progress Bar */}
            <div className="space-y-1.5">
              <div className="flex justify-between text-xs font-semibold">
                <span className="text-slate-400">Overall Utilization</span>
                <span className="text-white font-mono">
                  {formatBps(activeBudgetReport.overall_utilization_basis_points)}
                </span>
              </div>
              <div className="w-full h-3 bg-slate-800 rounded-full overflow-hidden">
                <div
                  className={`h-full transition-all duration-500 ${
                    activeBudgetReport.overall_utilization_basis_points >= 10000
                      ? 'bg-rose-500'
                      : activeBudgetReport.overall_utilization_basis_points >= 8000
                      ? 'bg-amber-500'
                      : 'bg-emerald-500'
                  }`}
                  style={{
                    width: `${Math.min(
                      100,
                      activeBudgetReport.overall_utilization_basis_points / 100
                    )}%`,
                  }}
                />
              </div>
            </div>
          </div>

          {/* Category Cards */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
            {activeBudgetReport.category_reports.map((cat) => {
              const pct = cat.utilization_basis_points / 100;
              const isExceeded = cat.status === 'Exceeded';
              const isWarning = cat.status === 'Warning';

              return (
                <div
                  key={cat.category}
                  className={`p-5 rounded-xl bg-[#111726] border transition flex flex-col justify-between space-y-4 ${
                    isExceeded
                      ? 'border-rose-500/50 shadow-rose-950/20'
                      : isWarning
                      ? 'border-amber-500/40'
                      : 'border-slate-800'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <h4 className="text-sm font-bold text-white">{cat.category}</h4>
                    {isExceeded ? (
                      <span className="flex items-center space-x-1 text-[10px] bg-rose-500/20 text-rose-400 px-2 py-0.5 rounded font-semibold border border-rose-500/30">
                        <Flame className="w-3 h-3" />
                        <span>Exceeded</span>
                      </span>
                    ) : isWarning ? (
                      <span className="flex items-center space-x-1 text-[10px] bg-amber-500/20 text-amber-400 px-2 py-0.5 rounded font-semibold border border-amber-500/30">
                        <AlertTriangle className="w-3 h-3" />
                        <span>Warning</span>
                      </span>
                    ) : (
                      <span className="flex items-center space-x-1 text-[10px] bg-emerald-500/20 text-emerald-400 px-2 py-0.5 rounded font-semibold border border-emerald-500/30">
                        <CheckCircle className="w-3 h-3" />
                        <span>On Track</span>
                      </span>
                    )}
                  </div>

                  <div className="space-y-1">
                    <div className="flex justify-between text-xs">
                      <span className="text-slate-400">Spent: {formatINR(cat.spent_paisa)}</span>
                      <span className="text-slate-300 font-mono">Limit: {formatINR(cat.allocated_paisa)}</span>
                    </div>
                    <div className="w-full h-2 bg-slate-800 rounded-full overflow-hidden">
                      <div
                        className={`h-full ${
                          isExceeded ? 'bg-rose-500' : isWarning ? 'bg-amber-500' : 'bg-emerald-500'
                        }`}
                        style={{ width: `${Math.min(100, pct)}%` }}
                      />
                    </div>
                  </div>

                  <div className="flex items-center justify-between text-[11px] pt-2 border-t border-slate-800/80">
                    <span className="text-slate-400">Remaining</span>
                    <span
                      className={`font-mono font-bold ${
                        cat.remaining_paisa < 0 ? 'text-rose-400' : 'text-slate-200'
                      }`}
                    >
                      {formatINR(cat.remaining_paisa)}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        <div className="text-center py-12 text-slate-500 text-sm">
          No active budget found. Click "New Budget Plan" above to configure.
        </div>
      )}

      {/* Modal: New Budget Plan */}
      {modalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <div className="bg-[#111726] border border-slate-800 rounded-xl max-w-lg w-full p-6 space-y-4">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-white">Create Budget Allocation Plan</h3>
              <button onClick={() => setModalOpen(false)} className="text-slate-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleCreateBudget} className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">Plan Name</label>
                  <input
                    type="text"
                    placeholder="e.g. November 2026 Household"
                    value={budgetName}
                    onChange={(e) => setBudgetName(e.target.value)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-300 mb-1">Period</label>
                  <select
                    value={period}
                    onChange={(e) => setPeriod(e.target.value as BudgetPeriod)}
                    className="w-full bg-slate-900 border border-slate-700 text-xs rounded-lg p-2.5 text-slate-200"
                  >
                    <option value="Monthly">Monthly</option>
                    <option value="Quarterly">Quarterly</option>
                    <option value="Annual">Annual</option>
                    <option value="Custom">Custom</option>
                  </select>
                </div>
              </div>

              {/* Category Limits */}
              <div className="space-y-2 border border-slate-800 rounded-lg p-3 bg-slate-900/40">
                <div className="flex justify-between items-center mb-1">
                  <span className="text-xs font-bold text-white uppercase tracking-wider">
                    Category Allocations
                  </span>
                  <button
                    type="button"
                    onClick={handleAddCategoryRow}
                    className="text-xs text-emerald-400 hover:text-emerald-300"
                  >
                    + Add Category
                  </button>
                </div>

                {categories.map((c, idx) => (
                  <div key={idx} className="grid grid-cols-12 gap-2 items-center">
                    <input
                      type="text"
                      placeholder="Category"
                      value={c.category}
                      onChange={(e) => handleCategoryChange(idx, 'category', e.target.value)}
                      className="col-span-5 bg-slate-900 border border-slate-700 text-xs rounded p-2 text-slate-200"
                      required
                    />
                    <input
                      type="number"
                      placeholder="₹ Limit"
                      value={c.allocatedRupees}
                      onChange={(e) => handleCategoryChange(idx, 'allocatedRupees', e.target.value)}
                      className="col-span-4 bg-slate-900 border border-slate-700 text-xs rounded p-2 text-slate-200 font-mono text-right"
                      required
                    />
                    <div className="col-span-3 flex items-center space-x-1">
                      <input
                        type="number"
                        min="10"
                        max="100"
                        placeholder="Alert %"
                        value={c.threshold}
                        onChange={(e) =>
                          handleCategoryChange(idx, 'threshold', parseInt(e.target.value) || 80)
                        }
                        className="w-full bg-slate-900 border border-slate-700 text-xs rounded p-2 text-slate-200 text-right"
                      />
                      <span className="text-[10px] text-slate-400">%</span>
                    </div>
                  </div>
                ))}
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
                  Save Budget Plan
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
