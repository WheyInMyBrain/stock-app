# 🏫 Greenwald Earnings Power Value (EPV) Module

This module implements a rolling **Earnings Power Value (EPV)** valuation pipeline based on the value-investing methodology developed by Professor Bruce Greenwald (Columbia Business School). It isolates a company's sustainable cash generation under a strict **zero-growth scenario ($g = 0$)**.

## 📊 Core Financial Philosophy

For highly cyclical commodity businesses (like metals, alloys, and mining), future growth assumptions are highly speculative. EPV addresses this uncertainty by stripping out future growth premiums and valuing the business based purely on its **current operations and historical operational efficiency**.

By comparing this no-growth baseline against your growth-driven models (DCF and Monte Carlo), you can calculate the exact valuation gap known as the **Value of Growth Premium**:

$$\text{Value of Growth Premium} = \text{Current Stock Price} - \text{EPV Baseline Share Price}$$

---

## 🖩 Mathematical Framework

The module scans your data chronologically and processes a rolling accounting normalization loop at every historical node index $k$:

### 1. Cyclically Adjusted Operating Earnings (Normalized EBIT)
Operating profit margins fluctuate heavily during economic cycles. The engine calculates the rolling mean operating efficiency ($\text{EBIT}$ margin) up to the current point in time ($0 \dots k$) and maps it to that period's active revenue base to determine the sustainable operating profit:

$$\text{Rolling Mean EBIT Margin } (\overline{m}_k) = \frac{1}{k+1} \sum_{j=0}^{k} \frac{\text{EBIT}_j}{\text{Revenue}_j}$$

$$\text{Normalized EBIT} = \text{Revenue}_k \times \overline{m}_k$$

*Note: $\text{EBIT}$ is reconstructed natively as $\text{Profit Before Tax (PBT)} + \text{Finance Costs}$.*

### 2. The Effective Corporate Tax Trap
To avoid artificial tax shelters or one-off penalty distortions, the effective tax rate is audited dynamically:

$$\text{Effective Tax Rate } (\tau) = \frac{\text{Tax Expense}_k}{\text{EBIT}_k}$$

$$\text{Audited Target Layer: } \text{If } \tau < 0\% \text{ or } \tau > 40\%, \text{ then } \tau \rightarrow 25.0\% \text{ (Standard Safe Proxy)}$$

### 3. Greenwald Maintenance CapEx Adjustment
In a zero-growth state, a company does not need to spend cash on expansion or capacity building. It only spends capital to maintain existing machinery. Therefore, **Total CapEx is split into Maintenance CapEx and Growth CapEx**. 

Over a long enough period, sustainable maintenance requirements structurally converge toward depreciation ($\text{CapEx} \rightarrow \text{Depreciation}$). The engine uses historical averages to filter out growth-oriented investments:

$$\text{Maintenance CapEx Ratio} = \min\left(1.0, \frac{\sum \text{Depreciation}}{\sum \text{Total CapEx}}\right)$$

$$\text{Normalized CapEx} = \text{Current CapEx}_k \times \text{Maintenance CapEx Ratio}$$

### 4. Normalized Free Cash Flow (Sustainable Earnings Power)
The true growth-free cash flow profile represents the sustainable cash stream available to clear capital requirements into infinity:

$$\text{Normalized FCF} = \left(\text{Normalized EBIT} \times (1 - \tau)\right) + \text{Depreciation}_k - \text{Normalized CapEx}$$

### 5. Perpetuity Capitalization and Equity Allocation
Because the growth rate is explicitly fixed at zero ($g=0$), the Gordon Growth formula simplifies into a flat perpetuity discounted directly by the Weighted Average Cost of Capital ($\text{WACC}$):

$$\text{Enterprise Value (EV}_{\text{EPV}}) = \frac{\text{Normalized FCF}}{\text{WACC}}$$

$$\text{EPV Fair Value Per Share} = \frac{\text{EV}_{\text{EPV}}}{\text{Chronological Shares Outstanding}}$$

---

## ⚙️ Data Input Keys
* **Operating Line Items:** `ProfitBeforeTax`, `FinanceCosts`, `DepreciationDepletionAndAmortisationExpense`, `TaxExpense`.
* **Capital Controls:** `PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities` (CapEx).
* **Corporate Shares Matching:** `NumberOfShares` + `ShareholdingPattern_ContextI`.