# M1 — Invent Distance

## Question and supplied world

The observations were four Pythagorean triples mapping `(x,y)` to a scalar.
The supplied grammar contained variables, constants, `+ - * / sqrt`, and
composition. It did not contain distance, norm, geometry, or the target
expression. Mathematics was represented as `W=(S,A,T,O)`: expressions were
states, grammar operations were actions, evaluation was transition, and the
input/output pairs were observations.

## What it figured out

Bottom-up expression enumeration, deduplicated by behavior on the training
points, found `sqrt((x*x)+(y*y))`. It was the first fitting behavior in the
deterministic size-ordered search: size 8 at discovery cost 99,573. The result
then predicted four unseen triples exactly, including `(28,45)->53`.

## Why it counted as ontology growth

The expression was retained as a callable concept. Four downstream predictions
then required four evaluations; the no-concept baseline required rediscovery
plus evaluation, cost 99,577. The eight-node expression replaced 24 raw
observation tokens, for compression gain 16.

## Controls, evidence, and limits

A non-Pythagorean control either produces no bounded fit or a fit that fails
held-out generalization. The record therefore says `proof_status=empirical`:
the experiment verifies training and transfer behavior, not uniqueness or a
formal theorem about Euclidean distance. Implementation: `discover_concept`,
`transfer_report`, and `compression_report` in `src/math_world.rs`.
