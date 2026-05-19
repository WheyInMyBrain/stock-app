# 🎲 Stochastic Monte Carlo Simulation Module

This module implements a probabilistic **Monte Carlo Simulation Engine** to analyze valuation risk under extreme uncertainty. Instead of looking at historical stock price fluctuations (market sentiment), this model extracts the **historical volatility of the business operations** and runs 10,000 randomized 5-year asset lifecycles.

## 📊 Core Analytical Theory

For cyclical commodity companies (like metals and mining), using single, fixed growth numbers can hide structural downside risks. This module converts your financial ledger inputs into dynamic **probability distributions**, simulating how commodity up-cycles and down-cycles disrupt value over time.

---

## 🖩 Mathematical Framework

### 1. Statistical Volatility Extraction
The engine scans your dynamic historical ledger array and derives the population mean ($\mu$) and standard deviation ($\sigma$) for both business growth rates and cash margins:

$$\text{Mean Growth } (\mu_g) = \frac{1}{N}\sum_{i=1}^N g_i \quad \text{, } \quad \text{Volatility } (\sigma_g) = \sqrt{\frac{1}{N}\sum_{i=1}^N (g_i - \mu_g)^2}$$

$$\text{Mean Margin } (\mu_m) = \frac{1}{N}\sum_{i=1}^N m_i \quad \text{, } \quad \text{Volatility } (\sigma_m) = \sqrt{\frac{1}{N}\sum_{i=1}^N (m_i - \mu_m)^2}$$

### 2. Gaussian Random Walk Randomization
During each of the 10,000 independent simulation trials, the engine samples a set of parameters using a continuous Gaussian (Normal) Distribution distribution map:

$$\text{Sampled Growth } (\hat{g}) \sim \mathcal{N}(\mu_g, \sigma_g)$$

$$\text{Sampled Margin } (\hat{m}) \sim \mathcal{N}(\mu_m, \sigma_m)$$

### 3. Iterative Valuation Loop
Every single trial computes a completely unique Enterprise Value profile using the sampled operational parameters:

$$\text{Enterprise Value (EV)} = \sum_{t=1}^{5} \frac{\text{Rev}_{FY25} \times (1 + \hat{g})^t \times \hat{m}}{(1 + \text{WACC})^t} + \frac{\left[\frac{\text{FCF}_5 \times (1 + g_n)}{\text{WACC} - g_n}\right]}{(1 + \text{WACC})^5}$$

### 4. Quantile Aggregation Spectrum
Once all 10,000 trials are executed, the final array of simulated stock prices is sorted in ascending order to extract precise percentile probability boundaries:

| Quantile | Real-World Market Meaning |
| :--- | :--- |
| **10th Percentile (P10)** | Severe Bear Case (Deep cyclical down-turn simulation). |
| **30th Percentile (P30)** | Conservative / Margin of Safety Pricing. |
| **50th Percentile (P50)** | Median Outlook (The high-probability meat of the curve). |
| **70th Percentile (P70)** | Optimistic / Cyclical Expansion. |
| **90th Percentile (P90)** | Aggressive Bull Case (Commodity super-cycle breakout). |

---

## 🛡️ Operational Safeguards
* **Liquidation Floor:** If a randomized trial samples an incredibly negative growth/margin combo that yields an implied negative enterprise value, the trial is instantly discarded ($\text{Price} > 0$) to protect the integrity of the data distribution.
* **API Standardization:** Outputs are mapped directly into an automated serialization struct to produce `bse_monte_carlo_projections.json`.