# Phase 4: UB Detection and Decision Log - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-05
**Phase:** 04-ub-detection-and-decision-log
**Areas discussed:** Sanitizer builds, has_ub labels & -O0 diff, Symbolic scan, Decisions log & gate, plus follow-ups (sizing, error codes, bulk sign, image, fixtures)

---

## Sanitizer builds

| Question | Options | Selected |
|---|---|---|
| Where ASan+UBSan runs | Host clang -m32 / Host LP64 / You decide | Host clang -m32 |
| Target trap UBSan | Yes, add trap UBSan / No, host only / You decide | Yes |
| MSan if no i386 | LP64 + unsigned-char fallback / Drop MSan / You decide | LP64 fallback |
| Host driver | Same roles, host runtime / libFuzzer-style harness / You decide | Same roles, host runtime |
| Pair fails host build | Log and continue / Fail the run / You decide | Log and continue |
| UBSan check set | undefined without unsigned / add unsigned as info / You decide | undefined without unsigned |
| Known-UB samples | In-repo C fixtures / Planted into test bed / You decide | In-repo fixtures |
| Replay size | Every vector / Capped per pair | Every vector |

## has_ub labels & -O0 diff

| Question | Options | Selected |
|---|---|---|
| Relabel mechanics | Regenerate vectors / Sidecar only / You decide | Regenerate vectors |
| Label grammar | source:kind / Bare source / You decide | source:kind |
| Opt-diff effect | Set has_ub + decision / Review flag only / You decide | Set has_ub + decision |
| -O0 builds | Both on target under QEMU / Host -m32 / You decide | Both on target |
| Clean label | Key absent + ub_checked sidecar / You decide | Key absent + ub_checked |
| Crash relabel | Refine to sanitizer label / Keep crash / You decide | Refine |
| Site key | Innermost target frame / Full stack hash / You decide | Innermost target frame |
| Host-only findings | Count / Target-confirmed only | Count |
| Timeouts | Log entry kind timeout / Outside log | Log entry |

## Symbolic scan

| Question | Options | Selected |
|---|---|---|
| Tool | KLEE only / Frama-C Eva / Both | KLEE only |
| KLEE ABI | Try -m32, fall back LP64 / LP64 directly / You decide | Try -m32, fall back LP64 |
| Symbolic inputs | Buffer up to N + declared scalars / Buffer fixed length / You decide | Buffer up to N + scalars |
| WR-13 | First plan of Phase 4 / Separate quick / Accept risk | First plan |
| KLEE scope | All drivable, floor 3 / Exactly 3 / You decide | All drivable, floor 3 |
| KLEE output | Log + witness vector / Log only / You decide | Log + witness |
| Budget | Instruction budget + wall cap / Wall-clock only / You decide | Instruction + wall cap |
| CI job | New ub job / Inside vectors / You decide | New ub job |
| Unconfirmed KLEE | Keep as klee-unconfirmed / Drop | Keep |

## Decisions log & gate

| Question | Options | Selected |
|---|---|---|
| Pre-fill | Proposed action ≠ decision / Pre-fill counts / You decide | Proposed ≠ decision |
| Log vs policy | Separate log, policy checked / Merge into policy / You decide | Separate log |
| Re-runs | Merge by key, never auto-delete / Regenerate and re-sign / You decide | Merge by key |
| Impl-defined facts | Section in same log / Separate facts file / You decide | Same log |
| Input class key | entry + label + witness / One per vector / You decide | entry + label + witness |
| Gate | mt ub check notice in CI / CI fails when undecided / You decide | mt ub check, notice |

## Follow-ups

| Question | Options | Selected |
|---|---|---|
| Sizing | Floor-first + cut list / Split into two phases / Accept overrun | Floor-first + cut list |
| Error codes | Tool proposes, human signs / Human writes all / You decide | Tool proposes |
| Bulk sign | Bulk allowed, per-entry record / One by one / You decide | Bulk allowed |
| Image | Early image plan + founder merge / You decide | Early image plan |
| Fixtures | Permanent CI self-test / Test once | Permanent self-test |

**User's choice:** the recommended option on every question; no free-text notes.

## Claude's Discretion

Budget values (N, KLEE instructions, wall cap), the label source order, kind and cause tokens, the crate split, the CLI beyond `check`/`sign`, and sharing between the host and target runtime templates.

## Deferred Ideas

UB manager UI (v2.0); Frama-C Eva; unsigned-wrap info notes; UB pass over fuzz corpora (Phase 5).
