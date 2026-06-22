---
description: One iteration of the WPT compliance loop — pick a tractable failure, find the root cause, fix it, and A/B verify with no regressions.
argument-hint: "[test-substring or suite, e.g. flex-minimum-size  or  css/css-grid]"
---

You are grinding Blitz toward WPT (Web Platform Tests) compliance, one fix at a time.
This command runs **one iteration** of the loop. Do the mechanical steps with the
helper, and apply judgment where it matters. Target for this run: **$ARGUMENTS**

## How Blitz renders (where fixes land)
Blitz is a thin engine over big crates. A failing CSS test is almost always one of:
- **`packages/blitz-dom`** — DOM, the Taffy glue, baselines, inline/text layout
  (`src/layout/mod.rs`, `src/layout/inline.rs`). Most in-repo fixes live here.
- **`packages/stylo_taffy/src/convert.rs`** — Stylo computed style → Taffy style.
  A missing/wrong property mapping shows up as many tests failing the same way.
  (Triple-licensed incl. MPL — keep that in mind.)
- **`taffy`** (sister repo `DioxusLabs/taffy`) — the actual flex/grid/block algorithm.
  Patch it locally via the commented `# taffy = { path = "../taffy" }` in `Cargo.toml`,
  rebuild, and the WPT loop tests your Taffy change against Blitz.
- An **upstream limitation** you can't fix here (e.g. Parley 0.10 `InlineBox` has no
  baseline field). When you hit one, say so and pick a different target.

## The loop — do these in order

1. **Establish a clean baseline.** Run the suite and freeze it:
   ```
   !./wpt/triage.sh run $ARGUMENTS && ./wpt/triage.sh snapshot before
   ```
   (No-arg `run` defaults to `css/css-flexbox css/css-grid`. Pass a suite to widen.)

2. **Pick a target.** `!./wpt/triage.sh targets`
   - Prefer **ATT** tests (exact `checkLayout` value checks) over **REF** (screenshot
     diffs) — ATT failures tell you the precise wrong number and are deterministic.
   - Prefer **near-pass** (smallest `fail` count) and **no hard flags**
     (W/D/S/M = writing-mode/direction/subgrid/masonry are deep rabbit holes).
   - **Ignore `grid-lanes`** — that's `display: grid-lanes`, unimplemented CSS Grid L3,
     a whole new algorithm, not a fix.
   - If $ARGUMENTS named a specific test, use that instead.

3. **Read the exact failure.** `!./wpt/triage.sh errors <test-substring>`
   You'll get lines like `data-offset-y expected 100 got 0`. Then open the test file
   under `wpt/tests/<path>` to see the CSS/HTML and what's actually being asserted.

4. **Find the root cause.** Trace the wrong value into the code (see "where fixes land").
   Read the relevant layout/convert code before editing. Confirm your hypothesis explains
   the *specific* numbers in the assertion.

5. **Fix it.** Make the smallest correct change. Match surrounding style. Guard for
   writing mode / direction when your change assumes horizontal-tb.

6. **A/B verify — this is mandatory.**
   ```
   !./wpt/triage.sh run $ARGUMENTS && ./wpt/triage.sh snapshot after && ./wpt/triage.sh compare before after
   ```
   Acceptance bar for a clean contribution:
   - **Net positive**, and ideally **zero regressions**.
   - **Any ATT (value-check) regression is a real bug in your fix — investigate, don't ship.**
   - A **REF (screenshot) regression** may be a latent bug your fix *exposed* (two
     wrong-but-matching renders now diverge). Explain each one; don't hand-wave it.
   - Sanity-check no collateral damage by widening once, e.g.
     `./wpt/triage.sh run css/css-align css/css-tables && ./wpt/triage.sh compare before after`
     after re-snapshotting against the original binary.

7. **Confirm hygiene & report.** `!cargo clippy -p blitz-dom && cargo fmt -p blitz-dom -- --check`
   Summarize: target, root cause, the diff, and the exact `compare` numbers
   (improved/regressed). Note whether it belongs upstream (Taffy) or in-repo, and
   whether anything is blocked on an upstream dep.

## Notes
- Artifacts live in `wpt/.triage/` (the runner wipes `wpt/output/` each run).
- For an A/B across a code change you can also drive git yourself: snapshot `before`
  on the current tree, `git stash`, build+snapshot, `git stash pop` — but the simplest
  flow is snapshot-before / edit / snapshot-after as above.
- Equivalent `just` recipes exist: `just wpt-run`, `wpt-targets`, `wpt-errors`,
  `wpt-snapshot`, `wpt-compare`.
