import {
  Account,
  BalanceSheet,
  Budget,
  BudgetReport,
  CashFlowStatement,
  IncomeStatement,
  SecurityStatus,
  TradeRecord,
  TradingSummary,
  Transaction,
  VaultDocumentSummary,
  VaultIntegrityReport,
} from '../types';

declare global {
  interface Window {
    __BHOOT_API_BASE__?: string;
  }
}

// Configurable API Base URL with secure airgapped localhost default
const API_BASE =
  (typeof window !== 'undefined' && window.__BHOOT_API_BASE__) ||
  (typeof window !== 'undefined' && window.location.origin.startsWith('http://127.0.0.1')
    ? window.location.origin
    : 'http://127.0.0.1:1420');

function getSessionToken(): string | null {
  try {
    return sessionStorage.getItem('bhoot_session_token');
  } catch {
    return null;
  }
}

function setSessionToken(token: string | null): void {
  try {
    if (token) {
      sessionStorage.setItem('bhoot_session_token', token);
    } else {
      sessionStorage.removeItem('bhoot_session_token');
    }
  } catch {
    // sessionStorage unavailable
  }
}

/**
 * Standard typed HTTP client communicating exclusively with the airgapped Rust backend.
 * Never falls back to mock or demo data in memory.
 */
async function httpRequest<T>(
  method: 'GET' | 'POST' | 'PUT' | 'DELETE',
  path: string,
  body?: unknown
): Promise<T> {
  const url = `${API_BASE}${path}`;
  const headers: Record<string, string> = {
    Accept: 'application/json',
  };

  const token = getSessionToken();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  let requestBody: string | undefined;
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
    requestBody = JSON.stringify(body);
  }

  const response = await fetch(url, {
    method,
    headers,
    body: requestBody,
  });

  if (!response.ok) {
    let errorDetail = `HTTP ${response.status} ${response.statusText}`;
    try {
      const errJson = await response.json();
      if (errJson && errJson.error) {
        errorDetail = errJson.error;
      }
    } catch {
      // response is not json
    }
    throw new Error(errorDetail);
  }

  // Handle empty responses
  const contentType = response.headers.get('content-type') || '';
  if (contentType.includes('application/json')) {
    return (await response.json()) as T;
  }

  const text = await response.text();
  return text as unknown as T;
}

export const api = {
  // System Health
  async getHealth(): Promise<{ status: string; os: string; version: string; airgap: boolean }> {
    return httpRequest('GET', '/api/health');
  },

  // Security Center & Authentication
  async getSecurityStatus(): Promise<SecurityStatus> {
    return httpRequest<SecurityStatus>('GET', '/api/security/status');
  },

  async enrollSession(pin: string, confirmPin: string): Promise<boolean> {
    const res = await httpRequest<{ success: boolean; session_token?: string }>(
      'POST',
      '/api/auth/enroll',
      { pin, confirm_pin: confirmPin }
    );
    if (res.success && res.session_token) {
      setSessionToken(res.session_token);
      return true;
    }
    return false;
  },

  async unlockSession(pin: string): Promise<boolean> {
    try {
      const res = await httpRequest<{ success: boolean; session_token?: string }>(
        'POST',
        '/api/auth/unlock',
        { pin }
      );
      if (res.success && res.session_token) {
        setSessionToken(res.session_token);
        return true;
      }
      return false;
    } catch {
      return false;
    }
  },

  async lockSession(): Promise<void> {
    try {
      await httpRequest('POST', '/api/auth/lock', {});
    } finally {
      setSessionToken(null);
    }
  },

  async setPin(currentPin: string, newPin: string, confirmNewPin?: string): Promise<void> {
    await httpRequest('POST', '/api/auth/set-pin', {
      current_pin: currentPin,
      new_pin: newPin,
      confirm_new_pin: confirmNewPin,
    });
  },

  // Chart of Accounts
  async getAccounts(): Promise<Account[]> {
    return httpRequest<Account[]>('GET', '/api/accounts');
  },

  async createAccount(acc: {
    name: string;
    account_type: string;
    subtype: string;
    currency?: string;
    description?: string;
  }): Promise<Account> {
    return httpRequest<Account>('POST', '/api/accounts', acc);
  },

  async updateAccountStatus(accountId: string, status: 'Active' | 'Archived' | 'Frozen'): Promise<void> {
    await httpRequest('POST', '/api/accounts/update-status', {
      account_id: accountId,
      status,
    });
  },

  async getAccountBalance(accountId: string): Promise<number> {
    const accounts = await this.getAccounts();
    const acc = accounts.find((a) => a.id === accountId);
    if (!acc) return 0;
    // Calculate balance from balance sheet
    const bs = await this.getBalanceSheet();
    const allItems = [...bs.assets, ...bs.liabilities, ...bs.equity];
    const item = allItems.find((i) => i.account_id === accountId);
    if (item) return item.balance_paisa ?? 0;
    // Check income statement for Income / Expense accounts
    try {
      const is = await this.getIncomeStatement();
      const incExp = [...is.operating_income, ...is.operating_expenses];
      const ieItem = incExp.find((i) => i.account_id === accountId);
      if (ieItem) return ieItem.balance_paisa ?? 0;
    } catch {
      // ignore
    }
    return 0;
  },

  // Double-Entry Transactions
  async getTransactions(limit = 200, offset = 0): Promise<Transaction[]> {
    return httpRequest<Transaction[]>('GET', `/api/transactions?limit=${limit}&offset=${offset}`);
  },

  async postTransaction(tx: {
    date?: string;
    tx_type: string;
    postings: { account_id: string; amount_paisa: number; direction: 'Debit' | 'Credit'; memo?: string }[];
    category?: string;
    payment_method?: string;
    reference?: string;
    notes?: string;
  }): Promise<Transaction> {
    return httpRequest<Transaction>('POST', '/api/transactions', tx);
  },

  async reverseTransaction(transactionId: string, reason: string): Promise<void> {
    await httpRequest('POST', '/api/transactions/reverse', {
      transaction_id: transactionId,
      reason,
    });
  },

  // Trading Journal
  async getTrades(): Promise<TradeRecord[]> {
    return httpRequest<TradeRecord[]>('GET', '/api/trades');
  },

  async getTradingSummary(): Promise<TradingSummary> {
    return httpRequest<TradingSummary>('GET', '/api/trades/summary');
  },

  async recordTrade(trade: {
    broker_account_id: string;
    trade_date?: string;
    symbol: string;
    trade_type: string;
    gross_amount_paisa: number;
    charges: {
      brokerage_paisa: number;
      stt_paisa: number;
      exchange_turnover_paisa: number;
      gst_paisa: number;
      sebi_paisa: number;
      stamp_duty_paisa: number;
    };
    net_amount_paisa: number;
    order_reference?: string;
  }): Promise<TradeRecord> {
    return httpRequest<TradeRecord>('POST', '/api/trades', trade);
  },

  // Budgets & Limits
  async getBudgets(): Promise<Budget[]> {
    return httpRequest<Budget[]>('GET', '/api/budgets');
  },

  async saveBudget(b: {
    id?: string;
    name: string;
    period: string;
    start_date: string;
    end_date: string;
    categories: { category: string; allocated_paisa: number; alert_threshold_pct: number }[];
    description?: string;
  }): Promise<Budget> {
    return httpRequest<Budget>('POST', '/api/budgets', b);
  },

  async evaluateBudget(id: string): Promise<BudgetReport | null> {
    return httpRequest<BudgetReport | null>('GET', `/api/budgets/evaluate?id=${encodeURIComponent(id)}`);
  },

  // Financial Reports
  async getBalanceSheet(asOf?: string): Promise<BalanceSheet> {
    const q = asOf ? `?as_of=${encodeURIComponent(asOf)}` : '';
    return httpRequest<BalanceSheet>('GET', `/api/reports/balance-sheet${q}`);
  },

  async getIncomeStatement(from?: string, to?: string): Promise<IncomeStatement> {
    const params = new URLSearchParams();
    if (from) params.set('from', from);
    if (to) params.set('to', to);
    const q = params.toString() ? `?${params.toString()}` : '';
    return httpRequest<IncomeStatement>('GET', `/api/reports/income-statement${q}`);
  },

  async getCashFlowStatement(from?: string, to?: string): Promise<CashFlowStatement> {
    const params = new URLSearchParams();
    if (from) params.set('from', from);
    if (to) params.set('to', to);
    const q = params.toString() ? `?${params.toString()}` : '';
    return httpRequest<CashFlowStatement>('GET', `/api/reports/cash-flow${q}`);
  },

  // Document Vault
  async getVaultDocuments(): Promise<VaultDocumentSummary[]> {
    return httpRequest<VaultDocumentSummary[]>('GET', '/api/vault/documents');
  },

  async uploadVaultDocument(filename: string, dataBase64: string, linkedTxId?: string): Promise<VaultDocumentSummary> {
    return httpRequest<VaultDocumentSummary>('POST', '/api/vault/upload', {
      filename,
      data_base64: dataBase64,
      linked_tx_id: linkedTxId || null,
    });
  },

  async downloadVaultDocument(id: string): Promise<{ id: string; filename: string; mime_type: string; data_base64: string }> {
    return httpRequest('GET', `/api/vault/download?id=${encodeURIComponent(id)}`);
  },

  async deleteVaultDocument(id: string): Promise<void> {
    await httpRequest('POST', '/api/vault/delete', { id });
  },

  async verifyVaultIntegrity(): Promise<VaultIntegrityReport> {
    return httpRequest<VaultIntegrityReport>('GET', '/api/vault/verify');
  },

  // Encrypted Backup & Disaster Recovery
  async createBackup(passphrase: string, destinationPath?: string): Promise<{ success: boolean; bytes_written: number; path: string }> {
    return httpRequest('POST', '/api/backup/create', {
      passphrase,
      destination_path: destinationPath || null,
    });
  },

  async restoreBackup(passphrase: string, backupFilePath: string): Promise<{ success: boolean; restored_bytes: number; integrity_verified: boolean }> {
    return httpRequest('POST', '/api/backup/restore', {
      passphrase,
      backup_file_path: backupFilePath,
    });
  },

  // Settings
  async getSettings(): Promise<{ currency: string; theme: string; auto_lock_minutes: number; offline_mode: boolean; financial_year_start: string }> {
    return httpRequest('GET', '/api/settings');
  },

  async updateSettings(settings: { currency: string; theme: string; auto_lock_minutes: number; offline_mode: boolean; financial_year_start: string }): Promise<void> {
    await httpRequest('POST', '/api/settings', settings);
  },

  // Audit Events
  async getAuditEvents(): Promise<any[]> {
    return httpRequest<any[]>('GET', '/api/audit/events');
  },

  // CSV Export Utility
  exportCsv(data: string, filename: string): void {
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
