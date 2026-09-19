# Testing `buzz-org-agent`

```bash
. ./bin/activate-hermit
cargo test -p buzz-org-agent
```

That one command is the crate's suite: E-1 fixture round-trips (compiled as
lib unit tests so the shipped binary does not link the loader), one unit
test per judge gate, one transition test per Org agent § 5.2 row, the
`50009` allow-list, and the pipeline proofs (outbox drain after disconnect;
newer generation mid-THINK → `stale`).

`cargo test -p buzz-org-agent --features fixtures` builds the same loader
the harness uses. The default binary is built without that feature.

A-1 does not start a relay. `run` / `dry-run` / `replay` / `doctor` are
flag-surface only; the proofs are the tests above.
