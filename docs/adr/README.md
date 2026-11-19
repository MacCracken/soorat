# Architecture Decision Records

Decisions about soorat — what we chose, the context, and the consequences we accept. Use these when a future reader would reasonably ask *"why did we do it this way?"*

## Conventions

- **Filename**: `NNNN-kebab-case-title.md`, zero-padded to four digits. Never renumber.
- **One decision per ADR.** If a decision supersedes a prior one, add a new ADR and set the old one's status to `Superseded by NNNN`.
- **Status lifecycle**: `Proposed` → `Accepted` → (optionally) `Superseded` or `Deprecated`.
- Use [`template.md`](template.md) as the starting point.

## ADR vs. architecture note vs. guide

| Kind | Lives in | Answers |
|---|---|---|
| ADR | `docs/adr/` | *Why did we choose X over Y?* |
| Architecture note | `docs/architecture/` | *What non-obvious constraint is true about the code?* |
| Guide | `docs/guides/` | *How do I do X?* |

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-cyrius-port-with-rust-oracle.md) | Port soorat to Cyrius, with the frozen Rust 1.2.0 crate as the oracle | Accepted; oracle clause superseded by 0003 |
| [0002](0002-port-api-divergences.md) | API divergences between Rust 1.2.0 and the Cyrius 2.0.0 port | Accepted (growing) |
| [0003](0003-correctness-is-the-oracle.md) | Correctness is the oracle; Rust 1.2.0 is the functionality reference | Accepted |
