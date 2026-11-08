export type AccountType = 'Asset' | 'Liability' | 'Equity' | 'Income' | 'Expense';

export type AccountSubtype =
  | 'BankAccount'
  | 'UPIWallet'
  | 'CashWallet'
  | 'BrokerAccount'
  | 'InvestmentAccount'
  | 'CreditCard'
  | 'PersonalLoan'
  | 'Payable'
  | 'OwnerCapital'
  | 'RetainedEarnings'
  | 'Salary'
  | 'BusinessRevenue'
  | 'TradingGain'
  | 'OtherIncome'
  | 'LivingExpense'
  | 'TradingLoss'
  | 'Tax'
  | 'OtherExpense';

export interface Account {
  id: string;
  name: string;
  account_type: AccountType;
  subtype: AccountSubtype;
  status: 'Active' | 'Frozen' | 'Archived';
  currency: string;
  description?: string;
  created_at: string;
  updated_at: string;
}

export type Direction = 'Debit' | 'Credit';

export interface Posting {
  account_id: string;
  amount_paisa: number;
  direction: Direction;
  memo?: string;
}

export type TransactionType =
  | 'Income'
  | 'Expense'
  | 'Transfer'
  | 'Deposit'
  | 'Withdrawal'
  | 'Trade'
  | 'Adjustment'
  | 'Reversal';

export type PaymentMethod =
  | 'UPI'
  | 'NetBanking'
  | 'DebitCard'
  | 'CreditCard'
  | 'Cash'
  | 'Cheque'
  | 'NEFT'
  | 'IMPS'
  | 'RTGS'
  | 'BrokerInternal'
  | 'Other';

export interface Transaction {
  id: string;
  timestamp: string;
  tx_type: TransactionType;
  postings: Posting[];
  category?: string;
  payment_method?: PaymentMethod;
  reference_number?: string;
  notes?: string;
  verification_status: 'Pending' | 'Verified' | 'Flagged' | 'Reconciled';
  reversal_of?: string;
}

export type TradeType =
  | 'IntradayEquity'
  | 'DeliveryEquity'
  | 'Futures'
  | 'Options'
  | 'Commodity'
  | 'Currency';

export interface TradeRecord {
  id: string;
  broker_account_id: string;
  trade_date: string;
  symbol: string;
  trade_type: TradeType;
  gross_pnl_paisa: number;
  total_charges_paisa: number;
  net_pnl_paisa: number;
  order_id?: string;
  contract_note_id?: string;
}

export interface TradingSummary {
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  win_rate_basis_points: number;
  gross_profit_paisa: number;
  gross_loss_paisa: number;
  total_charges_paisa: number;
  net_pnl_paisa: number;
  profit_factor_scaled: number;
  max_drawdown_paisa: number;
}

export interface PayoutRecord {
  id: string;
  broker_account_id: string;
  destination_bank_account_id: string;
  request_date: string;
  settlement_date?: string;
  requested_amount_paisa: number;
  tds_paisa: number;
  net_credited_paisa: number;
  utr_number?: string;
  status: 'Requested' | 'Settled' | 'Rejected';
}

export type BudgetPeriod = 'Monthly' | 'Quarterly' | 'Annual' | 'Custom';
export type BudgetStatus = 'OnTrack' | 'Warning' | 'Exceeded';

export interface CategoryBudget {
  category: string;
  allocated_paisa: number;
  alert_threshold_pct: number;
}

export interface CategoryBudgetReport {
  category: string;
  allocated_paisa: number;
  spent_paisa: number;
  remaining_paisa: number;
  utilization_basis_points: number;
  status: BudgetStatus;
}

export interface Budget {
  id: string;
  name: string;
  period: BudgetPeriod;
  start_date: string;
  end_date: string;
  categories: CategoryBudget[];
  notes?: string;
}

export interface BudgetReport {
  budget_id: string;
  budget_name: string;
  period: BudgetPeriod;
  total_allocated_paisa: number;
  total_spent_paisa: number;
  total_remaining_paisa: number;
  overall_utilization_basis_points: number;
  category_reports: CategoryBudgetReport[];
}

export interface AccountSummaryItem {
  account_id: string;
  name: string;
  subtype: AccountSubtype;
  balance_paisa: number;
}

export interface BalanceSheet {
  as_of: string;
  assets: AccountSummaryItem[];
  total_assets_paisa: number;
  liabilities: AccountSummaryItem[];
  total_liabilities_paisa: number;
  equity: AccountSummaryItem[];
  total_equity_paisa: number;
  retained_earnings_paisa: number;
  net_worth_paisa: number;
  is_balanced: boolean;
}

export interface IncomeStatement {
  from: string;
  to: string;
  operating_income: AccountSummaryItem[];
  total_operating_income_paisa: number;
  trading_gross_pnl_paisa: number;
  trading_charges_paisa: number;
  trading_net_pnl_paisa: number;
  total_income_paisa: number;
  operating_expenses: AccountSummaryItem[];
  total_operating_expenses_paisa: number;
  net_profit_paisa: number;
}

export interface CashFlowStatement {
  from: string;
  to: string;
  operating_cash_flow_paisa: number;
  investing_cash_flow_paisa: number;
  financing_cash_flow_paisa: number;
  net_cash_flow_paisa: number;
  beginning_cash_paisa: number;
  ending_cash_paisa: number;
}

export interface VaultDocumentSummary {
  id: string;
  filename: string;
  mime_type: string;
  file_size_bytes: number;
  blake3_hash: string;
  sha256_hash: string;
  created_at: string;
}

export interface VaultIntegrityReport {
  total_documents: number;
  verified_documents: number;
  corrupted_documents: number;
  corrupted_ids: string[];
}

export interface BackupManifest {
  version: number;
  created_at: string;
  payload_size_bytes: number;
  blake3_digest: string;
  argon2_m_cost: number;
  argon2_t_cost: number;
  argon2_p_cost: number;
}

export interface SecurityStatus {
  boot_mode: 'UEFI' | 'BIOS';
  secure_boot: boolean;
  luks2_encrypted: boolean;
  sqlcipher_active: boolean;
  audit_chain_length: number;
  audit_chain_verified: boolean;
  offline_firewall_active: boolean;
  session_locked: boolean;
  auth_enrolled: boolean;
}
