# 🐋 Merton-Bates Stochastic Jump-Diffusion Engine

### `analysis/src/merton_bates/`

---

## 📌 Overview
The **Merton-Bates Engine** is a high-performance quantitative asset-pricing black box written in native Rust. Unlike fundamental accounting engines (DCF, EPV) that focus strictly on corporate financial statement ledgers, this module models raw equity market microstructure physics. 

By ingesting daily timeline chart parquets natively, the engine maps continuous asset price trajectories, handles asymmetric volatility smile states, and derives predictive **Value-at-Risk (VaR)** boundaries using highly concurrent stochastic simulations.

---

## 🔍 Core Problem & Architecture Fixes

During the system audit, two major structural vulnerabilities were discovered and permanently resolved across all parallel pipeline frameworks:

### 1. The Quarterly Ingestion Flood Filter (The Gatekeeper)
* **The Bug:** The Indian corporate filing system records quarterly sheets as cumulative year-to-date (YTD) metrics. The old pipeline utilized partial filename string matching (`!file.contains("-30-Jun")`), which allowed quarterly nodes with variant scraper name formatting to leak past the filter. This inflated the timeline vectors from 5 clean years to 20 overlapping blocks, causing an impossible revenue CAGR spike to **42.32%** and artificially ballooning NSE valuations up to **2,093**.
* **The Fix:** We implemented a unified post-validation intercept. The engine now maps candidate tokens to standardized ISO string dates (`YYYY-MM-DD`) *upfront* on every row iteration. A strict calendar gatekeeper enforces that fields **must end in March (`-03-31`)** to be processed, dropping quarterly noise cleanly and snapping growth calculations back to a realistic **19.45%**.

### 2. Flat Vector Task Parallelization via Rayon
* **The Bug:** Processing 10 years of trading days sequentially meant running `2,456 days * 4 scenario configurations = 9,824` consecutive blocking simulation rounds, slowing down the runner module to **20.68 seconds** per execution track.
* **The Fix:** We decoupled the nested loop architecture by mapping every single day-to-scenario permutation into a structural coordinate vector called `evaluation_tasks`. Converting the outer processing framework to a parallel iterator (`.par_iter()`) allows **Rayon** to distribute the thousands of path-generation matrix tasks across all available CPU threads concurrently via work-stealing loops, reducing runtimes down to a fraction of a second.

---

## 📊 Mathematical Specifications

The asset price paths are compiled forward over a 63-trading-day horizon (1 fiscal quarter) by solving two coupled Stochastic Differential Equations (SDEs) using **Euler-Maruyama time-discretization stepping**:

### 1. Instantaneous Asset Price Dynamics (The Merton SDE)
$$dS_t = \mu S_t dt + \sqrt{V_t} S_t dW_t^1 + J S_t dN_t$$

* $\mu$: Conservative baseline drift coefficient representing the equity risk premium floor ($7.0\%$).
* $V_t$: Trailing continuous standard deviation wave derived dynamically via a **rolling 21-day log-return window** annualized by $\sqrt{252}$:
    $$\sigma_{\text{daily}} = \sqrt{\frac{1}{N-1}\sum_{i=1}^{N}(R_i - \bar{R})^2} \quad \longrightarrow \quad \sigma_{\text{annual}} = \sigma_{\text{daily}} \times \sqrt{252}$$
* $dW_t^1$: Standard Brownian motion Gaussian innovation vector ($Z \sim N(0, 1)$).
* $dN_t$: Independent Poisson counting process operating at a discrete scenario arrival rate $\lambda$ (`[1.0, 3.0]` jumps per calendar year).
* $J$: Asymmetric jump shock intensity mapping severe market gaps ($J = e^{Y} - 1$), where the jump magnitude is normally distributed around a structural negative crash target:
    $$Y \sim N(\mu_j, \sigma_j^2) \quad \text{with } \mu_j \in \{-0.10, +0.05\}, \, \sigma_j = 0.12$$

---

## 💾 Schema Matrix Mapping Outputs

Each structural scenario iteration dumps continuous matrix coordinates into your repository path array: `../data/{TICKER}/outputs/nse_merton_bates_credit_risk.json`.

```json
{
  "snapshot_date": "2024-03-31",
  "base_stock_price": 512.45,
  "implied_annual_volatility": 0.2456,
  "jump_intensity_lambda": 3.0,
  "expected_jump_size_mu_j": -0.10,
  "value_at_risk_95": 412.33,
  "value_at_risk_99": 318.12,
  "simulated_expected_value": 508.84
}