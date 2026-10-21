# Deferred items (phase 02)

Out-of-scope discoveries logged during plan execution; not fixed by the plan that found them.

| Found in | Item | Where |
|----------|------|-------|
| 02-21 | `cargo doc -p mt-capture` warns that public docs link to private items (pre-existing, not part of any pre-merge gate) | crates/mt-capture/src/capture.rs:346 (`REPORTED_TOOLS`), crates/mt-capture/src/compare.rs:28 (`strip_exclusions`) |
