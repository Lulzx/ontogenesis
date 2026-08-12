# M3 — Invent Square Numbers

## Question and restriction

From `1->1, 2->4, ..., 5->25`, the system had to infer a reusable unary
transformation. Its initial grammar contained only `+ - *`, constants, a
variable, and composition. `Square` is explicitly disabled during this search;
there is no exponentiation primitive.

## Search and discovery

`discover_square` enumerates unary expressions by size and deduplicates their
integer behavior on training inputs. It discovers `(n*n)` at size 3 and search
cost 6. It generalizes exactly to `6->36`, `9->81`, and `-3->9`. Only after
this discovery is `Square(argument)` admitted as a single reusable ontology
operator for later stages.

## Transfer measurement

The required transfer suite contains `x²+y²`, `(n+1)²-n²`, and the right side
of the odd-sum law. Across five occurrences, expanded multiplication costs 15
AST tokens while concept application costs 10, a gain of 5. This is a modest
but real representation saving, not merely curve fitting.

## Claim boundary

Status is empirical. Exact held-out values establish behavior on the tested
integers; they do not constitute a uniqueness proof over every possible
grammar extension. Tests also assert structurally that the discovered raw
expression is multiplication of the variable by itself.
