#!/bin/sh
# Certify emitted solutions with LamBench's own harness (Taelin's referee).
cd "$(dirname "$0")/lambench" && bun src/check.ts ../out
