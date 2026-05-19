# 📊 Corporate Multiples & Forensic Diagnostic Engine

This directory contains the high-performance parallel processing pipeline for computing institutional trading multiples, structural balance sheet ratios, capital efficiency diagnostics, and accounting forensic indicators. 

The module is designed to handle heterogeneous timelines, safely isolating high-resolution continuous operational flows from fragmented point-in-time balance sheet snapshots without data contamination.

---

## 🏗️ Core Architectural Layout

The engine runs a dual-tier analytical processing paradigm mapped onto multi-threaded data threads via Rayon:

### 🔹 Tier 1: Continuous Operational Ratios (Always Computed)
Calculated across all sequential reporting intervals ($N \approx 40$) using income statement, cash flow statement, and capital expenditure segments:

* **Operating EBIT Margin:** Measures primary core pricing leverage before capital structuring layers.
    $$\text{EBIT Margin} = \frac{\text{Profit Before Tax} + \text{Finance Costs}}{\text{Revenue From Operations}}$$
* **Net Profit Margin:** Post-tax ultimate bottom-line conversion efficiency.
    $$\text{Net Margin} = \frac{\text{Profit Before Tax} - \text{Tax Expense}}{\text{Revenue From Operations}}$$
* **Free Cash Flow (FCF) Margin %:** Measures structural cash generation capacity after asset infrastructure maintenance cycles.
    $$\text{FCF Margin} = \frac{\text{Cash Flows from Operating Activities} - |\text{CapEx}|}{\text{Revenue From Operations}}$$
* **Interest Coverage Capacity:** Safety margin tracking corporate debt serviceability limits.
    $$\text{Interest Coverage} = \frac{\text{EBIT}}{\text{Finance Costs}}$$
* **Accruals-to-Sales Intensity:** Traditional Total Accruals to Total Assets (TATA) variant tailored to identify high-risk non-cash revenue accounting spikes.
    $$\text{Accruals Intensity} = \frac{\text{Net Profit} - \text{Cash Flows from Operating Activities}}{\text{Revenue From Operations}}$$
* **Degree of Operating Leverage (DOL):** Measures the underlying earnings volatility and bottom-line multiplier sensitivity relative to topline scaling adjustments.
    $$\text{DOL} = \frac{\text{Revenue From Operations} - \text{Estimated Variable Costs}}{\text{EBIT}}$$
* **Breakeven Operating Revenue Point:** Calculates the minimum baseline threshold revenue scaling required to support fixed overhead operational expenses before net operational losses occur.
    $$\text{Breakeven Revenue} = \frac{\text{Estimated Fixed Costs}}{\text{Contribution Margin Ratio}}$$
* **CapEx-to-Depreciation Coverage Multiplier:** Audits corporate reinvestment health to confirm infrastructure assets are maintained sufficiently against real asset usage decay.
    $$\text{CapEx Coverage} = \frac{|\text{CapEx}|}{\text{Depreciation, Depletion \& Amortisation Expense}}$$
* **Estimated Infrastructure NBV Age (Years):** Measures the approximate lifespan remaining on the corporate physical infrastructure footprint (Net Book Value) before replacement cycles apply.
    $$\text{Estimated Plant Age} = \frac{\text{Property, Plant \& Equipment Net Base}}{\text{Depreciation Expense}}$$

---

### 🔹 Tier 2: Structural Snapshot Multipliers (Safely Guarded)
Triggered **only** when balance sheet entities are active ($N = 5$). Missing intervals return safe `None` (`null` tokens in serialized JSON outputs) to insulate core timelines from degradation:

* **Return on Invested Capital (RoIC):** Measures true business compounding power against all deployed operating hardware.
    $$\text{RoIC} = \frac{\text{EBIT} \times (1 - \tau)}{\text{Property, Plant \& Equipment} + (\text{Current Assets} - \text{Current Liabilities})}$$
    *(Where $\tau$ represents the dynamically computed effective tax rate bounded between $0\%$ and $45\%$.)*
* **Return on Equity (ROE):** Traditional DuPont top-level multi-stage equity efficiency proxy.
    $$\text{ROE} = \frac{\text{Net Profit}}{\text{Total Assets} - \text{Total Liabilities}}$$
* **Return on Assets (ROA):** Measures overall structural asset-utilization capability.
    $$\text{ROA} = \frac{\text{Net Profit}}{\text{Total Assets}}$$
* **Current / Quick Ratios:** Immediate liquid cover tracking systemic short-term solvency.
    $$\text{Current Ratio} = \frac{\text{Current Assets}}{\text{Current Liabilities}}$$
    $$\text{Quick Ratio} = \frac{\text{Current Assets} - \text{Inventories}}{\text{Current Liabilities}}$$
* **Inventory Turnover Multiplier:** Measures warehouse velocity, which is critical for highly cyclical commodity setups.
    $$\text{Inventory Turnover} = \frac{\text{Revenue From Operations}}{\text{Inventories}}$$
* **Cash Conversion Cycle (CCC) Days Proxy:** Tracks the speed at which capital moves through the supply chain.
    $$\text{CCC Days Proxy} = \left(\frac{\text{Inventories}}{\text{Revenue}} \times 365\right) + \left(\frac{\text{Trade Receivables}}{\text{Revenue}} \times 365\right)$$
* **Defensive Cash Burn Capacity (Months):** Quantifies total survival runways using only cash and high-liquidity assets if primary corporate operational revenues flatline completely.
    $$\text{Defensive Burn Window} = \frac{\text{Current Assets} - \text{Inventories}}{(\text{Total Expenses} - \text{Depreciation Expense}) / 12}$$
* **Net Liquidating Dissolution Cash:** Establishes the liquid capital distribution left over for common shareholders if operations cease instantly, liquid current assets are processed, and all outside liabilities are cleared.
    $$\text{Dissolution Cash Position} = \text{Current Assets} - \text{Total Liabilities}$$

---

## 🌊 Commodity Crash Inventory Haircut Impact Table

To stress-test macro cyclical asset shocks, the engine executes a multi-tiered structural asset write-down simulation matrix, evaluating overall asset degradation profiles when inventory elements encounter liquidation haircuts of $10\%$, $20\%$, $30\%$, $40\%$, and $50\%$:

$$\text{Simulated Post-Asset Value}_{(H)} = \text{Total Assets} - (\text{Inventories} \times H)$$
*(Where $H$ maps across the designated evaluation parameters: $H \in \{0.10, 0.20, 0.30, 0.40, 0.50\}$).*

---

## 💰 Valuation & Market Multipliers (Fuzzy Aligned)

By ingesting historical Unix-millisecond trading charts alongside share outstanding disclosures, the engine computes market capitalizations and enterprise valuations matched directly to financial report dates, automatically accounting for trading holidays via a 7-day calendar padding lookback loop:

* **Reconstructed Enterprise Value (EV):**
    $$EV = (\text{Stock Price} \times \text{Shares Outstanding}) + \text{Total Liabilities} - \text{Current Assets}$$
* **EV-to-EBITDA Multiplier:** Core trading valuation ratio independent of tax structures or local debt positions.
    $$\text{EV/EBITDA} = \frac{EV}{\text{EBIT} + \text{Depreciation \& Amortisation}}$$

---

## 🕵️‍♂️ Advanced Forensic Risk Matrix

### 1. Altman Z-Score (Manufacturing Sector Variant)
Predicts systemic corporate bankruptcy risk. A score $>2.99$ signals the **Safe Zone**, $1.81 \le Z \le 2.99$ maps the **Grey Zone**, and $<1.81$ points to imminent **Distress**.
$$Z = 1.2X_1 + 1.4X_2 + 3.3X_3 + 0.6X_4 + 0.999X_5$$
$$\begin{aligned}
X_1 &= \frac{\text{Current Assets} - \text{Current Liabilities}}{\text{Total Assets}} & X_2 &= \frac{\text{Net Equity} \times 0.65}{\text{Total Assets}} \\
X_3 &= \frac{\text{EBIT}}{\text{Total Assets}} & X_4 &= \frac{\text{Net Equity}}{\text{Total Liabilities}} \\
X_5 &= \frac{\text{Revenue From Operations}}{\text{Total Assets}}
\end{aligned}$$

### 2. Beneish M-Score (Accounting Manipulation Audit)
Calculates if financial results have been mathematically smoothed or distorted. Scores above $-1.78$ flag a high probability of earnings manipulation.
$$M = -4.84 + 0.92 \cdot \text{DSRI} + 0.528 \cdot \text{AQI} + 0.472 \cdot \text{SGI} + 0.404 \cdot \text{LVGI} + 0.1 \cdot \text{GMI}$$
* **DSRI (Days Sales in Receivables Index):** Tracks disproportionate growth in receivables vs revenue.
* **AQI (Asset Quality Index):** Measures shifts into non-traditional, capitalized asset items.
* **SGI (Sales Growth Index):** Controls for revenue growth patterns relative to baseline metrics.
* **LVGI (Leverage Index):** Catches climbing long-term structural dependencies.
* **GMI (Gross Margin Index):** Monitors structural changes in operating cost efficiency.

### 3. Piotroski F-Score (7-Point Modified Framework)
Tracks internal trend momentum based on fundamental directionality. The engine processes a 7-point scale using available trailing balance metrics:
1.  **ROA Profitability Check:** $+1$ if current $\text{ROA} > 0$.
2.  **Cash Generation Quality:** $+1$ if current $\text{CFO} > 0$.
3.  **Earnings Integrity:** $+1$ if current $\text{CFO} > \text{Net Income}$ (accrual protection check).
4.  **Profit Momentum:** $+1$ if current $\text{ROA} > \text{Previous ROA}$.
5.  **De-leveraging Momentum:** $+1$ if current $\frac{\text{Total Liabilities}}{\text{Total Assets}} < \text{Previous Ratio}$.
6.  **Liquidity Expansion:** $+1$ if current $\text{Current Ratio} > \text{Previous Current Ratio}$.
7.  **Asset Turn Acceleration:** $+1$ if current $\text{Asset Turnover} > \text{Previous Asset Turnover}$.

---

## 🚀 Execution Profile

The pipeline is compiled natively into the release workspace under optimal compiler constraints:
```bash
cargo run --release <TICKER>