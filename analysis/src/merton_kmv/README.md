# 🏛️ Merton-KMV Structural Distance-to-Default Engine

This standalone engine implements the institutional **Merton-KMV Structural Credit Risk Model**. Instead of relying on static accounting leverage ratios, it treats a corporation's equity value as a liquid European call option on its underlying, unobservable corporate asset base.

When the market value of the company's assets drops below its structural liabilities threshold (the default barrier), the call option expires out-of-the-money, signaling imminent structural default or bankruptcy.

---

## 📐 Underlying Quantitative Formulas

Because total asset value ($V_A$) and asset volatility ($\sigma_A$) cannot be directly observed in public markets, this engine takes liquid equity market capitalization ($E$) and trailing equity historical volatility ($\sigma_E$) to back them out simultaneously using the **Black-Scholes-Merton option framework**.

### 1. The Call Option Equivalence Form
$$E = V_A N(d_1) - e^{-rT} D N(d_2)$$

### 2. The Volatility Diffusion Constraint
$$\sigma_E = \left( \frac{V_A}{E} \right) N(d_1) \sigma_A$$

**Where:**
* $N(x)$ is the Standard Normal Cumulative Distribution Function (CDF).
* $r$ is the sovereign risk-free interest rate yield curve anchor.
* $T$ is the risk horizon window (typically set to 1.0 year).
* $D$ is the structural **KMV Default Barrier Trigger**, calculated as:

$$D = \text{Current Liabilities} + 0.5 \times \text{Non-Current Liabilities}$$

### 3. Calculating Distance to Default (DD) & Expected Default Frequency (EDF)
Once the simultaneous system converges, the unobservable variables are passed to extract the exact standard-deviation boundary distance and real probability of stress:

$$d_1 = \frac{\ln(V_A / D) + (r + 0.5\sigma_A^2)T}{\sigma_A \sqrt{T}}$$

$$\text{Distance to Default (DD)} = d_1$$

$$\text{Expected Default Frequency (EDF)} = N(-\text{DD})$$

---

## 🛠️ High-Performance Engine Design

The module runs zero-copy data picking inside the concurrent workspace pipeline:

### 🔄 1. Bisection Calendar Gap Search
Because corporate financial statement dates frequently land on weekends or holidays when public trading data is paused, the engine runs a **Bisection Binary Search Window** over the trailing `10Y.json` chart records. If an exact date doesn't match, it instantly scans backward to lock onto the closest available historical market pricing node.

### 🧮 2. 2D Newton-Raphson Non-Linear Solver
To solve the non-linear simultaneous equations for $V_A$ and $\sigma_A$, the engine computes the partial derivatives matrix (the Jacobian Matrix) for every historical statement slice in parallel across the Rayon thread-pool:

$$J = \begin{bmatrix} \frac{\partial f_1}{\partial V_A} & \frac{\partial f_1}{\partial \sigma_A} \\ \frac{\partial f_2}{\partial V_A} & \frac{\partial f_2}{\partial \sigma_A} \end{bmatrix} = \begin{bmatrix} N(d_1) & V_A \sqrt{T} N'(d_1) \\ \frac{N(d_1) \sigma_A}{V_A} & N(d_1) \end{bmatrix}$$

Values shift dynamically along the error vector trajectory gradient across a maximum threshold of 200 iterations or until error tolerances drop beneath a strict limit of $1\text{e-}5$.

---

## 📂 Output Data Schema

The calculated metrics serialize straight to pretty JSON vectors matching this specification layout:

```json
[
  {
    "snapshot_date": "2024-03-31",
    "equity_value_market_cap": 1254000000.0,
    "structural_default_barrier": 451000000.0,
    "inferred_asset_value": 1695420140.23,
    "inferred_asset_volatility": 0.2145,
    "distance_to_default_dd": 3.4215,
    "expected_default_frequency_edf": 0.00031
  }
]
```

### Interpretation Matrix
* **DD > 3.0:** Highly solvent capital structure. Deep cushion between asset base and debt obligations.
* **1.5 <= DD <= 3.0:** Moderate stress risk threshold. Monitor asset volatility expansion vectors.
* **DD < 1.5:** Structural Default Alert. Asset decay has breached safe parameters; expected default probability flags capital impairment.