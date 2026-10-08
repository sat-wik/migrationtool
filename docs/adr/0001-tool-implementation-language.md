---
type: ADR
status: accepted
date: 2026-10-07
deciders: Satwik
---

# ADR-0001: Write the migration tool in Rust

## Status

Accepted (LOCKED).

## Context

The tool must ship to on-prem and air-gapped customers (product spec, deployment section), drive c2rust, Hayroll, KLEE, Kani, sanitizers and fuzzers, and produce evidence bundles that third parties re-run. Options considered: Rust; Python; Rust core with a Python agent loop.

## Decision

The tool is written in Rust, as a Cargo workspace. External analysers (clang/LLVM, KLEE, c2rust, Hayroll, Kani, Bear) are invoked as pinned subprocesses inside a reproducible container, not linked as libraries. The LLM agent loop is also Rust, behind a model-agnostic provider trait so hosted and locally hosted open-weight models are interchangeable.

## Consequences

- One static binary per platform simplifies air-gapped installs and signed releases.
- The tool and its output share a toolchain, Clippy and rustfmt conventions.
- Agent-loop iteration is slower than in Python; mitigate with a small, well-tested provider and prompt-template layer.
- Python may still appear in throwaway research scripts under `research/`, never in the shipped tool.
