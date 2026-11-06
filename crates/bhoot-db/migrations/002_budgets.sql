-- BHOOT SQLCipher Schema Migration 002: Budgets and Category Allocations
-- Supports customizable spending limits, periodic tracking, and alert thresholds.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS budgets (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    period TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS category_budgets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    budget_id TEXT NOT NULL,
    category TEXT NOT NULL,
    allocated_paisa INTEGER NOT NULL CHECK (allocated_paisa >= 0),
    alert_threshold_pct INTEGER NOT NULL CHECK (alert_threshold_pct > 0 AND alert_threshold_pct <= 100),
    FOREIGN KEY (budget_id) REFERENCES budgets(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_category_budgets_bid ON category_budgets(budget_id);
