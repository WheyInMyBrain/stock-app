# 🎭 Multi-Regime Markov Switching Hidden State Engine

### `analysis/src/markov_regime/`

---

## 📌 Overview
The **Markov Regime Engine** is a native, highly parallelized hidden-state expectations tracking module. While classical DCF projection matrices assume corporate operations grow at linear constants forever, this engine models corporate parameters as a series of shifting, independent hidden economic regimes. 

By analyzing trailing operational data structures (Revenue Scale vs EBIT Margin boundaries), the engine solves localized state allocations and constructs predictive migration probability arrays.

---

## 📊 Mathematical Specifications

The platform maps data observations across $K = 3$ discrete operational state channels:
$$\text{State } S_t \in \{0: \text{Expansion Regime}, \, 1: \text{Stagnation Regime}, \, 2: \text{Systemic Crunch Regime}\}$$

The conditional probability transitions are optimized using a forward Baum-Welch expectation transition density framework mapped into an explicit stochastic matrix profile $P$:

$$P = \begin{pmatrix} 
p_{00} & p_{01} & p_{02} \\ 
p_{10} & p_{11} & p_{12} \\ 
p_{20} & p_{21} & p_{22} 
\end{pmatrix}$$

Where each coordinate element represents the formal probability parameter that a company will migrate from its current hidden state condition this fiscal year directly into an operational target state next period:
$$p_{ij} = P(S_{t+1} = j \mid S_t = i) \quad \text{subject to} \quad \sum_{j=0}^{2} p_{ij} = 1.0$$

---

## 💾 Schema Matrix Mapping Outputs

Calculated transition parameters map directly into target local database folders: `../data/{TICKER}/outputs/bse_markov_regime_transitions.json`.

```json
{
  "execution_snapshot_date": "2024-03-31",
  "extracted_current_state": 0,
  "margin_efficiency_growth_drift": 0.1982,
  "probability_matrix_transition_to_expansion": 0.70,
  "probability_matrix_transition_to_stagnation": 0.20,
  "probability_matrix_transition_to_crunch": 0.10
}