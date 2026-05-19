# 🎯 Discounted Cash Flow (DCF) Analytics Module

This module implements a dynamic, dual-horizon **Discounted Cash Flow (DCF)** valuation architecture utilizing the Polars framework. It isolates corporate cash-generation capacity across historical timelines and handles missing accounting lines using an automated indirect fallback ledger.

## 📊 Core Financial Mechanics

The valuation pipeline runs two distinct intrinsic value models simultaneously to cross-examine current asset pricing:

1. **Rolling Trend DCF:** Uses a moving window approach. For any historical year $t$, it calculates the compound growth rate and profit margins using only data up to that specific point in time ($0 \dots t$). This simulates what a real-time analyst would have calculated at that year-end.
2. **Omniscient Terminal DCF:** Applies fixed global forward boundaries derived across the entire multi-year data block to analyze structural stability.

---

## 🖩 Mathematical Framework

### 1. Free Cash Flow (FCF) Reconstruction
Free Cash Flow to the Firm (FCFF) represents the cash available to all capital providers after operating expenses and capital investments are cleared.

$$\text{FCF} = \text{Cash Flow from Operations (CFO)} - \text{Capital Expenditures (CapEx)}$$

#### 🔄 The Indirect Fallback Engine
If direct statements contain blank entries ($\text{CFO} = 0$ or $\text{CapEx} = 0$), the engine automatically reconstructs the cash line natively using an indirect accounting ledger structure:

$$\text{Estimated CFO} = \text{Profit Before Tax} + \text{Depreciation} - \text{Tax Expense} - \text{Finance Costs}$$

$$\text{Estimated CapEx} = \text{RevenueFromOperations} \times \overline{\left(\frac{\text{Historical CapEx}}{\text{Historical Revenue}}\right)}$$

### 2. Trailing Parameter Disclosures
For the Rolling Trend track, parameters compound dynamically at every node index $k$:

$$\text{Rolling Growth Rate } (g_k) = \frac{1}{k} \sum_{j=1}^{k} \frac{\text{Rev}_j - \text{Rev}_{j-1}}{\text{Rev}_{j-1}}$$

$$\text{Rolling FCF Margin } (m_k) = \frac{1}{k+1} \sum_{j=0}^{k} \frac{\text{FCF}_j}{\text{Rev}_j}$$

### 3. Inherent Present Value Horizon
The company's operational capacity is projected out for a 5-year discrete phase, discounted back to the present using the Weighted Average Cost of Capital ($\text{WACC}$):

$$\text{PV of Discrete Cash Flows} = \sum_{t=1}^{5} \frac{\text{Rev}_0 \times (1 + g)^t \times m}{(1 + \text{WACC})^t}$$

### 4. Terminal Value Boundary Conditions
To capture the value of the business beyond year 5 into infinity, the engine deploys the **Gordon Growth Model**. The terminal cash flow is capitalized using a permanent perpetual growth rate ($g_n$):

$$\text{Terminal Value (TV)} = \frac{\text{FCF}_5 \times (1 + g_n)}{\text{WACC} - g_n}$$

$$\text{PV of Terminal Value} = \frac{\text{TV}}{(1 + \text{WACC})^5}$$

### 5. Final Equity Allocation Price
The total Enterprise Value ($\text{EV}$) is aggregated and divided by the strict matching chronological share capital base row extracted from the shareholding parquets:

$$\text{Intrinsic Value Per Share} = \frac{\text{PV of Discrete Cash Flows} + \text{PV of Terminal Value}}{\text{Shares Outstanding}}$$

---

## ⚙️ Data Input Keys
* **Primary Identifier:** `tag_name == "RevenueFromOperations"` (and corresponding statement tags).
* **Double-Key Share Linkage:** `tag_name == "NumberOfShares"` paired with `context_id == "ShareholdingPattern_ContextI"`.