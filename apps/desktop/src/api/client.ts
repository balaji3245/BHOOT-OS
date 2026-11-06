import {
  Account,
  BalanceSheet,
  Budget,
  BudgetReport,
  CashFlowStatement,
  IncomeStatement,
  PayoutRecord,
  Posting,
  SecurityStatus,
  TradeRecord,
  TradingSummary,
  Transaction,
  VaultDocumentSummary,
  VaultIntegrityReport,
} from '../types';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const isTauri = () => typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;

async function tauriInvoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const tauri = (window as any).__TAURI__?.core;
  if (tauri?.invoke) {
    return tauri.invoke(cmd, args);
  }
  throw new Error('Tauri core not available');
}

// In-Memory Seed State for Offline Standalone / Virtual Verification
let mockAccounts: Account[] = [
  {
    id: 'acc-hdfc-01',
    name: 'HDFC Savings Bank',
    account_type: 'Asset',
    subtype: 'BankAccount',
    status: 'Active',
    currency: 'INR',
    description: 'Primary salary and operational account',
    created_at: new Date(Date.now() - 60 * 86400000).toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'acc-zerodha-01',
    name: 'Zerodha Trading & Demat',
    account_type: 'Asset',
    subtype: 'BrokerAccount',
    status: 'Active',
    currency: 'INR',
    description: 'NSE / BSE Derivatives & Equity',
    created_at: new Date(Date.now() - 45 * 86400000).toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'acc-icici-cc-01',
    name: 'ICICI Coral Credit Card',
    account_type: 'Liability',
    subtype: 'CreditCard',
    status: 'Active',
    currency: 'INR',
    description: 'Monthly utility expenses',
    created_at: new Date(Date.now() - 30 * 86400000).toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'acc-salary-inc-01',
    name: 'Consulting Income',
    account_type: 'Income',
    subtype: 'Salary',
    status: 'Active',
    currency: 'INR',
    description: 'Direct client retainer & salary',
    created_at: new Date(Date.now() - 60 * 86400000).toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'acc-rent-exp-01',
    name: 'Office & Studio Rent',
    account_type: 'Expense',
    subtype: 'LivingExpense',
    status: 'Active',
    currency: 'INR',
    description: 'Monthly fixed workspace rent',
    created_at: new Date(Date.now() - 60 * 86400000).toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'acc-groceries-exp-01',
    name: 'Household Groceries',
    account_type: 'Expense',
    subtype: 'LivingExpense',
    status: 'Active',
    currency: 'INR',
    description: 'Daily provisions and food',
    created_at: new Date(Date.now() - 60 * 86400000).toISOString(),
    updated_at: new Date().toISOString(),
  },
];

let mockTransactions: Transaction[] = [
  {
    id: 'tx-oct-salary-01',
    timestamp: new Date(Date.now() - 25 * 86400000).toISOString(),
    tx_type: 'Income',
    postings: [
      { account_id: 'acc-hdfc-01', amount_paisa: 25000000, direction: 'Debit', memo: 'October Retainer' },
      { account_id: 'acc-salary-inc-01', amount_paisa: 25000000, direction: 'Credit', memo: 'Consulting Revenue' },
    ],
    category: 'Consulting',
    payment_method: 'NEFT',
    reference_number: 'CMS987654321',
    verification_status: 'Verified',
  },
  {
    id: 'tx-nov-rent-01',
    timestamp: new Date(Date.now() - 5 * 86400000).toISOString(),
    tx_type: 'Expense',
    postings: [
      { account_id: 'acc-rent-exp-01', amount_paisa: 4500000, direction: 'Debit', memo: 'November Rent' },
      { account_id: 'acc-hdfc-01', amount_paisa: 4500000, direction: 'Credit', memo: 'Rent Transfer' },
    ],
    category: 'Rent',
    payment_method: 'UPI',
    reference_number: 'UPI2026110199',
    verification_status: 'Verified',
  },
  {
    id: 'tx-groceries-01',
    timestamp: new Date(Date.now() - 2 * 86400000).toISOString(),
    tx_type: 'Expense',
    postings: [
      { account_id: 'acc-groceries-exp-01', amount_paisa: 1450000, direction: 'Debit', memo: 'Supermarket' },
      { account_id: 'acc-icici-cc-01', amount_paisa: 1450000, direction: 'Credit', memo: 'Card swipe' },
    ],
    category: 'Groceries',
    payment_method: 'CreditCard',
    verification_status: 'Pending',
  },
];

let mockTrades: TradeRecord[] = [
  {
    id: 'tr-001',
    broker_account_id: 'acc-zerodha-01',
    trade_date: new Date(Date.now() - 10 * 86400000).toISOString(),
    symbol: 'NIFTY26NOV24500CE',
    trade_type: 'Options',
    gross_pnl_paisa: 3200000,
    total_charges_paisa: 145000,
    net_pnl_paisa: 3055000,
    order_id: 'ORD20261011A',
  },
  {
    id: 'tr-002',
    broker_account_id: 'acc-zerodha-01',
    trade_date: new Date(Date.now() - 4 * 86400000).toISOString(),
    symbol: 'BANKNIFTY26NOV52000PE',
    trade_type: 'Options',
    gross_pnl_paisa: -1200000,
    total_charges_paisa: 98000,
    net_pnl_paisa: -1298000,
    order_id: 'ORD20261102B',
  },
  {
    id: 'tr-003',
    broker_account_id: 'acc-zerodha-01',
    trade_date: new Date(Date.now() - 1 * 86400000).toISOString(),
    symbol: 'RELIANCE-EQ',
    trade_type: 'DeliveryEquity',
    gross_pnl_paisa: 1850000,
    total_charges_paisa: 120000,
    net_pnl_paisa: 1730000,
    order_id: 'ORD20261105C',
  },
];

let mockPayouts: PayoutRecord[] = [
  {
    id: 'pay-001',
    broker_account_id: 'acc-zerodha-01',
    destination_bank_account_id: 'acc-hdfc-01',
    request_date: new Date(Date.now() - 8 * 86400000).toISOString(),
    settlement_date: new Date(Date.now() - 7 * 86400000).toISOString(),
    requested_amount_paisa: 2000000,
    tds_paisa: 0,
    net_credited_paisa: 2000000,
    utr_number: 'UTR2026NOV001',
    status: 'Settled',
  },
];

let mockBudgets: Budget[] = [
  {
    id: 'bgt-nov-2026',
    name: 'November 2026 Household & Studio',
    period: 'Monthly',
    start_date: new Date(Date.now() - 6 * 86400000).toISOString(),
    end_date: new Date(Date.now() + 24 * 86400000).toISOString(),
    categories: [
      { category: 'Rent', allocated_paisa: 5000000, alert_threshold_pct: 90 },
      { category: 'Groceries', allocated_paisa: 2000000, alert_threshold_pct: 75 },
      { category: 'Utilities', allocated_paisa: 1000000, alert_threshold_pct: 80 },
    ],
    notes: 'Q4 Budget allocation',
  },
];

let mockVaultDocs: VaultDocumentSummary[] = [
  {
    id: 'doc-001',
    filename: 'Zerodha_ContractNote_26Nov.pdf',
    mime_type: 'Pdf',
    file_size_bytes: 245890,
    blake3_hash: '3f5a89b...e4a1',
    sha256_hash: '9a7c3b2...11d0',
    created_at: new Date(Date.now() - 3 * 86400000).toISOString(),
  },
  {
    id: 'doc-002',
    filename: 'Office_Rent_Agreement_2026.pdf',
    mime_type: 'Pdf',
    file_size_bytes: 1420800,
    blake3_hash: 'c81e9d0...88ff',
    sha256_hash: '55b0a31...77aa',
    created_at: new Date(Date.now() - 40 * 86400000).toISOString(),
  },
];

let mockSecurity: SecurityStatus = {
  boot_mode: 'UEFI',
  secure_boot: true,
  luks2_encrypted: true,
  sqlcipher_active: true,
  audit_chain_length: 42,
  audit_chain_verified: true,
  offline_firewall_active: true,
  session_locked: false,
};

export const api = {
  // Accounts
  async getAccounts(): Promise<Account[]> {
    if (isTauri()) return tauriInvoke('get_accounts');
    return [...mockAccounts];
  },

  async createAccount(acc: Omit<Account, 'id' | 'created_at' | 'updated_at'>): Promise<Account> {
    if (isTauri()) return tauriInvoke('create_account', { account: acc });
    const newAcc: Account = {
      ...acc,
      id: `acc-${Date.now()}`,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    mockAccounts.push(newAcc);
    return newAcc;
  },

  async getAccountBalance(id: string): Promise<number> {
    if (isTauri()) return tauriInvoke('get_account_balance', { id });
    const acc = mockAccounts.find((a) => a.id === id);
    if (!acc) return 0;
    let balance = 0;
    for (const tx of mockTransactions) {
      for (const p of tx.postings) {
        if (p.account_id === id) {
          if (acc.account_type === 'Asset' || acc.account_type === 'Expense') {
            balance += p.direction === 'Debit' ? p.amount_paisa : -p.amount_paisa;
          } else {
            balance += p.direction === 'Credit' ? p.amount_paisa : -p.amount_paisa;
          }
        }
      }
    }
    return balance;
  },

  // Transactions
  async getTransactions(limit = 50, offset = 0): Promise<Transaction[]> {
    if (isTauri()) return tauriInvoke('get_transactions', { limit, offset });
    return [...mockTransactions]
      .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
      .slice(offset, offset + limit);
  },

  async postTransaction(tx: {
    tx_type: Transaction['tx_type'];
    postings: Posting[];
    category?: string;
    payment_method?: Transaction['payment_method'];
    reference_number?: string;
    notes?: string;
  }): Promise<Transaction> {
    // Validate balanced invariant: sum(Debit) == sum(Credit)
    let debits = 0;
    let credits = 0;
    for (const p of tx.postings) {
      if (p.direction === 'Debit') debits += p.amount_paisa;
      else credits += p.amount_paisa;
    }
    if (debits !== credits) {
      throw new Error(`Transaction unbalanced: Debits (${debits}) != Credits (${credits})`);
    }

    if (isTauri()) return tauriInvoke('post_transaction', { tx });

    const newTx: Transaction = {
      ...tx,
      id: `tx-${Date.now()}`,
      timestamp: new Date().toISOString(),
      verification_status: 'Verified',
    };
    mockTransactions.unshift(newTx);
    mockSecurity.audit_chain_length += 1;
    return newTx;
  },

  async reverseTransaction(originalId: string, reason: string): Promise<Transaction> {
    if (isTauri()) return tauriInvoke('reverse_transaction', { originalId, reason });

    const orig = mockTransactions.find((t) => t.id === originalId);
    if (!orig) throw new Error('Transaction not found');
    if (orig.tx_type === 'Reversal') throw new Error('Cannot reverse a reversal transaction');
    if (mockTransactions.some((t) => t.reversal_of === originalId)) {
      throw new Error('Transaction already reversed');
    }

    const reversedPostings: Posting[] = orig.postings.map((p) => ({
      account_id: p.account_id,
      amount_paisa: p.amount_paisa,
      direction: p.direction === 'Debit' ? 'Credit' : 'Debit',
      memo: `Reversal: ${reason}`,
    }));

    const revTx: Transaction = {
      id: `rev-${Date.now()}`,
      timestamp: new Date().toISOString(),
      tx_type: 'Reversal',
      postings: reversedPostings,
      category: orig.category,
      notes: reason,
      verification_status: 'Verified',
      reversal_of: originalId,
    };
    mockTransactions.unshift(revTx);
    mockSecurity.audit_chain_length += 1;
    return revTx;
  },

  // Trading Journal
  async getTrades(brokerAccountId?: string): Promise<TradeRecord[]> {
    if (isTauri()) return tauriInvoke('get_trades', { brokerAccountId });
    if (brokerAccountId) {
      return mockTrades.filter((t) => t.broker_account_id === brokerAccountId);
    }
    return [...mockTrades];
  },

  async recordTrade(trade: Omit<TradeRecord, 'id'>): Promise<TradeRecord> {
    if (isTauri()) return tauriInvoke('record_trade', { trade });
    const newTrade: TradeRecord = {
      ...trade,
      id: `tr-${Date.now()}`,
    };
    mockTrades.unshift(newTrade);
    return newTrade;
  },

  async getTradingSummary(brokerAccountId?: string): Promise<TradingSummary> {
    if (isTauri()) return tauriInvoke('get_trading_summary', { brokerAccountId });

    const trades = brokerAccountId
      ? mockTrades.filter((t) => t.broker_account_id === brokerAccountId)
      : mockTrades;

    let wins = 0;
    let losses = 0;
    let grossProfit = 0;
    let grossLoss = 0;
    let totalCharges = 0;
    let netPnl = 0;

    for (const t of trades) {
      totalCharges += t.total_charges_paisa;
      netPnl += t.net_pnl_paisa;
      if (t.net_pnl_paisa > 0) {
        wins += 1;
        grossProfit += t.net_pnl_paisa;
      } else if (t.net_pnl_paisa < 0) {
        losses += 1;
        grossLoss += Math.abs(t.net_pnl_paisa);
      }
    }

    const total = trades.length;
    const winRate = total > 0 ? Math.round((wins / total) * 10000) : 0;
    const profitFactor = grossLoss > 0 ? Math.round((grossProfit / grossLoss) * 100) : 9999;

    return {
      total_trades: total,
      winning_trades: wins,
      losing_trades: losses,
      win_rate_basis_points: winRate,
      gross_profit_paisa: grossProfit,
      gross_loss_paisa: grossLoss,
      total_charges_paisa: totalCharges,
      net_pnl_paisa: netPnl,
      profit_factor_scaled: profitFactor,
      max_drawdown_paisa: 1298000,
    };
  },

  // Payouts
  async getPayouts(): Promise<PayoutRecord[]> {
    if (isTauri()) return tauriInvoke('get_payouts');
    return [...mockPayouts];
  },

  async recordPayout(p: Omit<PayoutRecord, 'id'>): Promise<PayoutRecord> {
    if (isTauri()) return tauriInvoke('record_payout', { payout: p });
    const newPayout: PayoutRecord = {
      ...p,
      id: `pay-${Date.now()}`,
    };
    mockPayouts.unshift(newPayout);
    return newPayout;
  },

  // Budgets
  async getBudgets(): Promise<Budget[]> {
    if (isTauri()) return tauriInvoke('get_budgets');
    return [...mockBudgets];
  },

  async saveBudget(b: Omit<Budget, 'id'> & { id?: string }): Promise<Budget> {
    if (isTauri()) return tauriInvoke('save_budget', { budget: b });
    const id = b.id || `bgt-${Date.now()}`;
    const budget: Budget = { ...b, id };
    const idx = mockBudgets.findIndex((x) => x.id === id);
    if (idx >= 0) {
      mockBudgets[idx] = budget;
    } else {
      mockBudgets.unshift(budget);
    }
    return budget;
  },

  async evaluateBudget(id: string): Promise<BudgetReport | null> {
    if (isTauri()) return tauriInvoke('evaluate_budget', { id });
    const budget = mockBudgets.find((b) => b.id === id);
    if (!budget) return null;

    let totalAllocated = 0;
    let totalSpent = 0;

    const catReports = budget.categories.map((cat) => {
      totalAllocated += cat.allocated_paisa;
      // Calculate spent from mock transactions
      let spent = 0;
      for (const tx of mockTransactions) {
        if (tx.tx_type === 'Expense' && tx.category?.toLowerCase() === cat.category.toLowerCase()) {
          for (const p of tx.postings) {
            if (p.direction === 'Debit') spent += p.amount_paisa;
          }
        }
      }
      totalSpent += spent;
      const remaining = cat.allocated_paisa - spent;
      const bps = cat.allocated_paisa > 0 ? Math.round((spent / cat.allocated_paisa) * 10000) : 0;
      const thresholdBps = cat.alert_threshold_pct * 100;

      let status: BudgetReport['category_reports'][0]['status'] = 'OnTrack';
      if (bps >= 10000) status = 'Exceeded';
      else if (bps >= thresholdBps) status = 'Warning';

      return {
        category: cat.category,
        allocated_paisa: cat.allocated_paisa,
        spent_paisa: spent,
        remaining_paisa: remaining,
        utilization_basis_points: bps,
        status,
      };
    });

    const overallBps = totalAllocated > 0 ? Math.round((totalSpent / totalAllocated) * 10000) : 0;

    return {
      budget_id: budget.id,
      budget_name: budget.name,
      period: budget.period,
      total_allocated_paisa: totalAllocated,
      total_spent_paisa: totalSpent,
      total_remaining_paisa: totalAllocated - totalSpent,
      overall_utilization_basis_points: overallBps,
      category_reports: catReports,
    };
  },

  // Financial Reports
  async getBalanceSheet(asOf?: string): Promise<BalanceSheet> {
    if (isTauri()) return tauriInvoke('get_balance_sheet', { asOf });

    const assets = [
      { account_id: 'acc-hdfc-01', name: 'HDFC Savings Bank', subtype: 'BankAccount' as const, balance_paisa: 20550000 },
      { account_id: 'acc-zerodha-01', name: 'Zerodha Trading & Demat', subtype: 'BrokerAccount' as const, balance_paisa: 3487000 },
    ];
    const liabilities = [
      { account_id: 'acc-icici-cc-01', name: 'ICICI Coral Credit Card', subtype: 'CreditCard' as const, balance_paisa: 1450000 },
    ];
    const equity = [
      { account_id: 'acc-equity-01', name: 'Owner Capital', subtype: 'OwnerCapital' as const, balance_paisa: 10000000 },
    ];

    const totalAssets = assets.reduce((s, a) => s + a.balance_paisa, 0);
    const totalLiabilities = liabilities.reduce((s, l) => s + l.balance_paisa, 0);
    const totalEquity = equity.reduce((s, e) => s + e.balance_paisa, 0);
    const netWorth = totalAssets - totalLiabilities;
    const retainedEarnings = netWorth - totalEquity;

    return {
      as_of: asOf || new Date().toISOString(),
      assets,
      total_assets_paisa: totalAssets,
      liabilities,
      total_liabilities_paisa: totalLiabilities,
      equity,
      total_equity_paisa: totalEquity,
      retained_earnings_paisa: retainedEarnings,
      net_worth_paisa: netWorth,
      is_balanced: true,
    };
  },

  async getIncomeStatement(from: string, to: string): Promise<IncomeStatement> {
    if (isTauri()) return tauriInvoke('get_income_statement', { from, to });

    const opIncome = [
      { account_id: 'acc-salary-inc-01', name: 'Consulting Income', subtype: 'Salary' as const, balance_paisa: 25000000 },
    ];
    const opExpenses = [
      { account_id: 'acc-rent-exp-01', name: 'Office Rent', subtype: 'LivingExpense' as const, balance_paisa: 4500000 },
      { account_id: 'acc-groceries-exp-01', name: 'Groceries', subtype: 'LivingExpense' as const, balance_paisa: 1450000 },
    ];

    const totalOpInc = opIncome.reduce((s, i) => s + i.balance_paisa, 0);
    const totalOpExp = opExpenses.reduce((s, e) => s + e.balance_paisa, 0);
    const tradeGross = 3850000;
    const tradeCharges = 363000;
    const tradeNet = tradeGross - tradeCharges;
    const totalIncome = totalOpInc + tradeNet;
    const netProfit = totalIncome - totalOpExp;

    return {
      from,
      to,
      operating_income: opIncome,
      total_operating_income_paisa: totalOpInc,
      trading_gross_pnl_paisa: tradeGross,
      trading_charges_paisa: tradeCharges,
      trading_net_pnl_paisa: tradeNet,
      total_income_paisa: totalIncome,
      operating_expenses: opExpenses,
      total_operating_expenses_paisa: totalOpExp,
      net_profit_paisa: netProfit,
    };
  },

  async getCashFlowStatement(from: string, to: string): Promise<CashFlowStatement> {
    if (isTauri()) return tauriInvoke('get_cash_flow_statement', { from, to });

    return {
      from,
      to,
      operating_cash_flow_paisa: 22537000,
      investing_cash_flow_paisa: -2000000,
      financing_cash_flow_paisa: 0,
      net_cash_flow_paisa: 20537000,
      beginning_cash_paisa: 3500000,
      ending_cash_paisa: 24037000,
    };
  },

  // Vault
  async getVaultDocuments(): Promise<VaultDocumentSummary[]> {
    if (isTauri()) return tauriInvoke('get_vault_documents');
    return [...mockVaultDocs];
  },

  async verifyVaultIntegrity(): Promise<VaultIntegrityReport> {
    if (isTauri()) return tauriInvoke('verify_vault_integrity');
    return {
      total_documents: mockVaultDocs.length,
      verified_documents: mockVaultDocs.length,
      corrupted_documents: 0,
      corrupted_ids: [],
    };
  },

  // Security Center
  async getSecurityStatus(): Promise<SecurityStatus> {
    if (isTauri()) return tauriInvoke('get_security_status');
    return { ...mockSecurity };
  },

  async lockSession(): Promise<void> {
    if (isTauri()) return tauriInvoke('lock_session');
    mockSecurity.session_locked = true;
  },

  async unlockSession(pin: string): Promise<boolean> {
    if (isTauri()) return tauriInvoke('unlock_session', { pin });
    if (pin.length >= 4) {
      mockSecurity.session_locked = false;
      return true;
    }
    return false;
  },

  // Export
  exportCsv(data: string, filename: string) {
    const blob = new Blob([data], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.setAttribute('href', url);
    link.setAttribute('download', filename);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  },
};
