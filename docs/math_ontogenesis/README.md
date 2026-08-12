# Mathematical Ontogenesis — Milestone Record

This directory is the milestone-by-milestone scientific record for the
Mathematical Ontogenesis ladder. Each file answers four questions: what the
stage was allowed to know, what the system actually searched or inferred,
what evidence supports the result, and what is explicitly not claimed.

Status is intentionally asymmetric. M1–M10 are reached experiments. M11–M30
are unreached milestones whose files document their dependency chain and the
evidence required before they may be marked complete. A complete directory is
not a claim of a completed ladder.

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
| [M11](M11_EUCLID_PROOF.md) | Euclid's proof | boundary — not reached |
| [M12](M12_SQRT2_CONTRADICTION.md) | Irrationality of sqrt(2) | blocked by M11 substrate |
| [M13](M13_VIETA_RELATIONS.md) | Polynomial root relations | not attempted |
| [M14](M14_SYMMETRY.md) | Symmetry | not attempted |
| [M15](M15_FOURIER.md) | Fourier representation | not attempted |
| [M16](M16_TOY_SPECTRAL_THEOREM.md) | Toy spectral theorem | not attempted |
| [M17](M17_EULER_PRODUCT.md) | Finite Euler product | not attempted |
| [M18](M18_TOY_ZETA.md) | Toy zeta | not attempted |
| [M19](M19_FUNCTIONAL_EQUATION.md) | Functional equation | not attempted |
| [M20](M20_COMPLETED_OBJECT.md) | Completed toy object | not attempted |
| [M21](M21_CRITICAL_LOCUS.md) | Critical symmetry locus | not attempted |
| [M22](M22_HIDDEN_ZEROS.md) | Hidden zeros | not attempted |
| [M23](M23_TOY_RH.md) | Toy RH conjecture | not attempted |
| [M24](M24_TOY_RH_EQUIVALENCE.md) | Toy-RH equivalence | not attempted |
| [M25](M25_TOY_RH_OBJECT.md) | RH-making toy object | not attempted |
| [M26](M26_REAL_ZETA_COMPLETION.md) | Real zeta completion | not attempted |
| [M27](M27_CRITICAL_LINE.md) | Critical line | not attempted |
| [M28](M28_RH_EQUIVALENCES.md) | New RH equivalences | not attempted |
| [M29](M29_RH_MAKING_OBJECT.md) | RH-making object | not attempted |
| [M30](M30_RIEMANN_HYPOTHESIS.md) | Riemann Hypothesis | not reached |

Reproduce reached stages with:

```sh
cargo test -p supsearch --lib math_world
cargo run --release --example math_world
cargo test --workspace
```
