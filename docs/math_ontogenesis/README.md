# Mathematical Ontogenesis — Milestone Record

This directory is the milestone-by-milestone scientific record for the
Mathematical Ontogenesis ladder. Each file answers four questions: what the
stage was allowed to know, what the system actually searched or inferred,
what evidence supports the result, and what is explicitly not claimed.

Status is intentionally asymmetric. M1–M12 are reached experiments, M13b,
M14c, M15b, M16, M17, M18, M19, M20, M21, M22, M23, and M24 have integrity-
calibrated L3 results, M25 has a toy RH-making object result, M15 has an exact
L2 result with a failed L3 gate, M26 has an exact L2 real-zeta completion
selection result, M27b has a finite-data L2 critical-locus conjecture, and
M28 has an exact L2 reflection-orbit equivalence. M29–M30 remain unreached. A
complete directory is not a claim of a completed ladder.

| Milestone | Topic | Status |
|---|---|---|
| [M1](M01_INVENT_DISTANCE.md) | Distance | reached — empirical |
| [M2](M02_CIRCLE_INVARIANT.md) | Circle invariant | reached — empirical |
| [M3](M03_SQUARE_NUMBERS.md) | Square numbers | reached — empirical |
| [M4](M04_ODD_SUM_LAW.md) | Odd-sum law | reached — conjectured |
| [M5](M05_INDUCTION.md) | Induction | reached — proof schema verified |
| [M6](M06_TELESCOPING.md) | Telescoping | reached — identity verified |
| [M7](M07_GCD_INVARIANT.md) | GCD invariant | reached — bounded verified |
| [M8](M08_GENERATING_FUNCTION.md) | Generating function | reached — formal-series verified |
| [M9](M09_EIGENVECTORS.md) | Eigenvectors | reached — bounded verified |
| [M10](M10_EQUIVALENT_STATEMENT.md) | Equivalent theorem statement | reached — formally checked modular |
| [M11](M11_EUCLID_PROOF.md) | Euclid's proof | reached — checked finite-list schema |
| [M12](M12_SQRT2_CONTRADICTION.md) | Irrationality of sqrt(2) | reached — checked valuation contradiction |
| [M13](M13_VIETA_RELATIONS.md) | Polynomial root relations | M13b reached — exact L2 |
| [M14](M14_SYMMETRY.md) | Symmetry | M14c reached — exact L3 |
| [M15](M15_FOURIER.md) | Fourier representation | attempted — exact L2; L3 gate failed |
| M15b | Fourier coordinate routing | reached — exact L3 conditional routing |
| [M16](M16_TOY_SPECTRAL_THEOREM.md) | Toy spectral theorem | reached — exact L3 conditional routing |
| [M17](M17_EULER_PRODUCT.md) | Finite Euler product | reached — exact L3 finite Euler product |
| [M18](M18_TOY_ZETA.md) | Toy zeta | reached — exact L3 toy zeta object |
| [M19](M19_FUNCTIONAL_EQUATION.md) | Functional equation | reached — exact L3 toy functional equation |
| [M20](M20_COMPLETED_OBJECT.md) | Completed toy object | reached — exact L3 toy completed object |
| [M21](M21_CRITICAL_LOCUS.md) | Critical symmetry locus | reached — exact L3 toy critical locus |
| [M22](M22_HIDDEN_ZEROS.md) | Hidden zeros | reached — exact L3 hidden toy zeros |
| [M23](M23_TOY_RH.md) | Toy RH conjecture | reached — exact L3 toy conjecture (conjectured) |
| [M24](M24_TOY_RH_EQUIVALENCE.md) | Toy-RH equivalence | reached — exact L3 toy equivalence |
| [M25](M25_TOY_RH_OBJECT.md) | RH-making toy object | reached — exact L3 toy RH-making object |
| [M26](M26_REAL_ZETA_COMPLETION.md) | Real zeta completion | reached — exact L2 completion selection |
| [M27](M27_CRITICAL_LINE.md) | Critical line | M27b reached — finite-data L2 conjecture |
| [M28](M28_RH_EQUIVALENCES.md) | RH equivalence | reached — exact L2 local reformulation |
| [M29](M29_RH_MAKING_OBJECT.md) | RH-making object | not attempted |
| [M30](M30_RIEMANN_HYPOTHESIS.md) | Riemann Hypothesis | not reached |

Reproduce reached stages with:

```sh
cargo test -p supsearch --lib math_world
cargo run --release --example math_world
cargo test --workspace
```
