import React, { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Budget, BudgetPeriod, BudgetReport } from '../types';
import { formatINR, formatBps } from '../utils/format';
import { Plus, AlertTriangle, CheckCircle, Flame, X, Trash2 } from 'lucide-react';

export const BudgetsView: React.FC = () => {
  const [budgets, setBudgets] = useState<Budget[]>([]);
  const [activeBudgetReport, setActiveBudgetReport] = useState<BudgetReport | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [loading, setLoading] = useState(true);

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
  const [isSubmitting, setIsSubmitting] = useState(false);

  const loadData = async () => {
    try {
      const bgts = await api.getBudgets();
      setBudgets(bgts);
      if (bgts.length > 0) {
        const rep = await api.evaluateBudget(bgts[0].id);
        setActiveBudgetReport(rep);
      } else {
        setActiveBudgetReport(null);
      }
    } catch (err) {
      console.error('Failed to load budgets:', err);
    } finally {
      setLoading(false);
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

  const handleRemoveCategoryRow = (idx: number) => {
    if (categories.length <= 1) return;
    setCategories(categories.filter((_, i) => i !== idx));
  };

  const handleCategoryChange = (idx: number, field: string, val: any) => {
    const updated = [...categories];
    updated[idx] = { ...updated[idx], [field]: val };
    setCategories(updated);
  };

  const handleCreateBudget = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!budgetName.trim()) return;

    setIsSubmitting(true);
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
      await loadData();
    } catch (err: any) {
      alert(`Failed to save budget: ${err.message}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <div className="text-white animate-pulse text-xs tracking-wider uppercase font-mono">
          Loading Budgets & Limits...
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-8 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-extrabold text-white tracking-tight">Budgets & Limits</h1>
          <p className="text-xs text-neutral-400">
            Periodic spending thresholds with automated early warning alerts
          </p>
        </div>
        <button
          onClick={() => setModalOpen(true)}
          className="flex items-center space-x-1.5 px-4 py-2 bg-white hover:bg-neutral-200 text-black text-xs font-semibold rounded-lg shadow-sm transition"
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
                  ? 'bg-white text-black border-white'
                  : 'bg-[#181818] text-neutral-400 border-[#333333] hover:text-white'
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
          <div className="p-6 rounded-xl bg-[#111111] border border-[#222222] space-y-4">
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
              <div>
                <h3 className="text-lg font-bold text-white">{activeBudgetReport.budget_name}</h3>
                <p className="text-xs text-neutral-400">
                  Plan Period: <span className="text-white">{activeBudgetReport.period}</span>
                </p>
              </div>
              <div className="flex items-center space-x-6 text-xs font-mono">
                <div>
                  <span className="text-neutral-400 block text-[10px] uppercase">Allocated</span>
                  <span className="text-sm font-bold text-white">
                    {formatINR(activeBudgetReport.total_allocated_paisa)}
                  </span>
                </div>
                <div>
                  <span className="text-neutral-400 block text-[10px] uppercase">Spent</span>
                  <span className="text-sm font-bold text-white">
                    {formatINR(activeBudgetReport.total_spent_paisa)}
                  </span>
                </div>
                <div>
                  <span className="text-neutral-400 block text-[10px] uppercase">Remaining</span>
                  <span className="text-sm font-bold text-white">
                    {formatINR(activeBudgetReport.total_remaining_paisa)}
                  </span>
                </div>
              </div>
            </div>

            {/* Overall Progress Bar */}
            <div className="space-y-1.5">
              <div className="flex justify-between text-xs font-semibold">
                <span className="text-neutral-400">Overall Utilization</span>
                <span className="text-white font-mono">
                  {formatBps(activeBudgetReport.overall_utilization_basis_points)}
                </span>
              </div>
              <div className="w-full h-2.5 bg-[#181818] border border-[#222222] rounded-full overflow-hidden">
                <div
                  className={`h-full transition-all duration-500 ${
                    activeBudgetReport.overall_utilization_basis_points >= 10000
                      ? 'bg-red-500'
                      : activeBudgetReport.overall_utilization_basis_points >= 8000
                      ? 'bg-amber-500'
                      : 'bg-white'
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
                  className={`p-5 rounded-xl bg-[#111111] border transition flex flex-col justify-between space-y-4 ${
                    isExceeded
                      ? 'border-red-500/50'
                      : isWarning
                      ? 'border-amber-500/40'
                      : 'border-[#222222]'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <h4 className="text-sm font-bold text-white">{cat.category}</h4>
                    {isExceeded ? (
                      <span className="flex items-center space-x-1 text-[10px] bg-red-500/10 text-red-400 px-2 py-0.5 rounded font-semibold border border-red-500/30">
                        <Flame className="w-3 h-3" />
                        <span>Exceeded</span>
                      </span>
                    ) : isWarning ? (
                      <span className="flex items-center space-x-1 text-[10px] bg-amber-500/10 text-amber-400 px-2 py-0.5 rounded font-semibold border border-amber-500/30">
                        <AlertTriangle className="w-3 h-3" />
                        <span>Warning</span>
                      </span>
                    ) : (
                      <span className="flex items-center space-x-1 text-[10px] bg-neutral-800 text-white px-2 py-0.5 rounded font-semibold border border-neutral-700">
                        <CheckCircle className="w-3 h-3" />
                        <span>On Track</span>
                      </span>
                    )}
                  </div>

                  <div className="space-y-1">
                    <div className="flex justify-between text-xs">
                      <span className="text-neutral-400">Spent: {formatINR(cat.spent_paisa)}</span>
                      <span className="text-neutral-300 font-mono">Limit: {formatINR(cat.allocated_paisa)}</span>
                    </div>
                    <div className="w-full h-2 bg-[#181818] border border-[#222222] rounded-full overflow-hidden">
                      <div
                        className={`h-full ${
                          isExceeded ? 'bg-red-500' : isWarning ? 'bg-amber-500' : 'bg-white'
                        }`}
                        style={{ width: `${Math.min(100, pct)}%` }}
                      />
                    </div>
                  </div>

                  <div className="flex items-center justify-between text-[11px] pt-2 border-t border-[#222222]">
                    <span className="text-neutral-400">Remaining</span>
                    <span
                      className={`font-mono font-bold ${
                        cat.remaining_paisa < 0 ? 'text-red-400' : 'text-white'
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
        <div className="p-12 text-center rounded-xl bg-[#111111] border border-[#222222] space-y-3">
          <p className="text-xs text-neutral-400">No active budget plans established.</p>
          <button
            onClick={() => setModalOpen(true)}
            className="px-4 py-2 bg-white text-black text-xs font-semibold rounded-lg hover:bg-neutral-200 transition"
          >
            Create Your First Budget
          </button>
        </div>
      )}

      {/* Modal: New Budget Plan */}
      {modalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4 overflow-y-auto">
          <div className="bg-[#111111] border border-[#222222] rounded-xl max-w-lg w-full p-6 space-y-4 my-8 shadow-2xl">
            <div className="flex items-center justify-between border-b border-[#222222] pb-3">
              <h3 className="text-base font-bold text-white">Establish Budget Plan</h3>
              <button onClick={() => setModalOpen(false)} className="text-neutral-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleCreateBudget} className="space-y-4">
              <div>
                <label className="block text-xs font-semibold text-neutral-300 mb-1">Plan Name</label>
                <input
                  type="text"
                  placeholder="e.g. Monthly Operating Budget"
                  value={budgetName}
                  onChange={(e) => setBudgetName(e.target.value)}
                  className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                  required
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-neutral-300 mb-1">Frequency Period</label>
                <select
                  value={period}
                  onChange={(e) => setPeriod(e.target.value as BudgetPeriod)}
                  className="w-full bg-[#181818] border border-[#333333] text-xs rounded-lg p-2.5 text-white focus:border-white focus:outline-none"
                >
                  <option value="Weekly">Weekly</option>
                  <option value="Monthly">Monthly</option>
                  <option value="Quarterly">Quarterly</option>
                  <option value="Annual">Annual</option>
                </select>
              </div>

              <div className="space-y-2 pt-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-white uppercase tracking-wider">
                    Category Allocations
                  </span>
                  <button
                    type="button"
                    onClick={handleAddCategoryRow}
                    className="text-xs text-white hover:text-neutral-300 font-semibold"
                  >
                    + Add Category
                  </button>
                </div>

                <div className="space-y-2">
                  {categories.map((c, idx) => (
                    <div key={idx} className="flex items-center space-x-2 bg-[#181818] p-2 rounded-lg border border-[#222222]">
                      <input
                        type="text"
                        placeholder="Category Name"
                        value={c.category}
                        onChange={(e) => handleCategoryChange(idx, 'category', e.target.value)}
                        className="flex-1 bg-[#111111] border border-[#333333] text-xs rounded p-2 text-white focus:border-white focus:outline-none"
                        required
                      />
                      <input
                        type="number"
                        placeholder="₹ Limit"
                        value={c.allocatedRupees}
                        onChange={(e) => handleCategoryChange(idx, 'allocatedRupees', e.target.value)}
                        className="w-24 bg-[#111111] border border-[#333333] text-xs rounded p-2 text-white font-mono focus:border-white focus:outline-none"
                        required
                      />
                      <input
                        type="number"
                        min="10"
                        max="100"
                        placeholder="Alert %"
                        value={c.threshold}
                        onChange={(e) => handleCategoryChange(idx, 'threshold', parseInt(e.target.value) || 80)}
                        className="w-16 bg-[#111111] border border-[#333333] text-xs rounded p-2 text-white font-mono focus:border-white focus:outline-none text-center"
                        title="Alert threshold percentage"
                        required
                      />
                      {categories.length > 1 && (
                        <button
                          type="button"
                          onClick={() => handleRemoveCategoryRow(idx)}
                          className="p-1.5 text-neutral-500 hover:text-red-400 transition"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  ))}
                </div>
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
                  {isSubmitting ? 'Saving...' : 'Save Budget'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
