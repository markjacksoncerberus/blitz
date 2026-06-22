#!/usr/bin/env bash
#
# wpt/triage.sh — a small, reusable loop for grinding down WPT failures in Blitz.
#
# The Blitz WPT runner (crate `wpt`) renders each web-platform-test and compares it
# against its reference (REF, screenshot diff) or its `checkLayout` assertions
# (ATT, exact layout-value checks). This script wraps the mechanical parts of the
# iterate loop so each step is one command: run, rank tractable targets, read the
# exact failing assertions, and A/B a code change for improvements/regressions.
#
# Typical loop (see `.claude/commands/wpt.md` for the full guided version):
#
#   ./wpt/triage.sh run                      # build + run (default: flexbox + grid)
#   ./wpt/triage.sh snapshot before          # freeze the pre-change result
#   ./wpt/triage.sh targets                  # pick a near-pass ATT test to fix
#   ./wpt/triage.sh errors <test-substring>  # see exact "expected X got Y" lines
#   # ...edit code...
#   ./wpt/triage.sh run && ./wpt/triage.sh snapshot after
#   ./wpt/triage.sh compare before after     # net improvements / regressions
#
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$REPO_DIR/wpt/output"          # wiped by the runner on every run
TRIAGE_DIR="$REPO_DIR/wpt/.triage"      # our artifacts live here so the wipe can't eat them
WPT_DIR="${WPT_DIR:-$REPO_DIR/wpt/tests}"
CLEAN_LOG="$TRIAGE_DIR/run.clean.log"
REPORT_JSON="$OUT_DIR/wptreport.json"
mkdir -p "$TRIAGE_DIR"
DEFAULT_SUITES=(css/css-flexbox css/css-grid)

# Strip ANSI colour codes and OSC-8 terminal hyperlinks from runner output.
strip_ansi() { sed -r 's/\x1b\][^\x1b]*\x1b\\//g; s/\x1b\[[0-9;]*[mGKHJ]//g'; }

ensure_wpt_dir() {
  if [ ! -d "$WPT_DIR" ]; then
    echo "ERROR: WPT tests not found at $WPT_DIR" >&2
    echo "Clone them first:" >&2
    echo "  git clone --depth 1 --single-branch https://github.com/web-platform-tests/wpt $WPT_DIR" >&2
    exit 1
  fi
}

cmd_run() {
  ensure_wpt_dir
  local suites=("$@")
  [ ${#suites[@]} -eq 0 ] && suites=("${DEFAULT_SUITES[@]}")
  echo ">> building wpt runner..." >&2
  (cd "$REPO_DIR" && cargo build -rp wpt) >&2
  echo ">> running: ${suites[*]}" >&2
  WPT_DIR="$WPT_DIR" "$REPO_DIR/target/release/wpt" "${suites[@]}" \
    | strip_ansi | tee "$CLEAN_LOG" \
    | grep -E "tests (FOUND|PASSED|FAILED|SKIPPED|RUN)|subtests PASSED" || true
  echo ">> full log: $CLEAN_LOG   report: $REPORT_JSON" >&2
}

# Parse the "Ordered Results" lines:
#   [NNNN/total] STATUS (pass/total) name (ms) KIND (FLAGS)
# into TSV: name<TAB>status<TAB>pass<TAB>total<TAB>kind<TAB>flags
parse_results() {
  [ -f "$CLEAN_LOG" ] || { echo "No run found. Run: $0 run" >&2; exit 1; }
  grep -E '^\[[0-9]{4}/' "$CLEAN_LOG" | sed -E \
    's#^\[[0-9]+/[0-9]+\] ([A-Z]+) \(([0-9]+)/([0-9]+)\) ([^ ]+) \([0-9]+ms\) ([A-Z]+)( \(([A-Z]+)\))?.*#\4\t\1\t\2\t\3\t\5\t\7#' \
    | sort -u
}

cmd_targets() {
  local limit="${1:-30}"
  echo "=== Failure buckets by sub-directory (excluding grid-lanes = unimplemented CSS Grid L3) ==="
  parse_results | awk -F'\t' '$2=="FAIL" && $1 !~ /grid-lanes/ {
      d=$1; sub(/\/[^\/]+$/,"",d); print d }' | sort | uniq -c | sort -rn | head -15
  echo ""
  echo "=== Top $limit tractable ATT targets (near-pass first; exact-value checkLayout tests) ==="
  echo "    Excludes grid-lanes. Flags: F=float I=intrinsic C=calc D=direction W=writing-mode S=subgrid M=masonry"
  echo "    fail  pass/total  flags  test"
  parse_results | awk -F'\t' '
    $2=="FAIL" && $5=="ATT" && $1 !~ /grid-lanes/ && $3+0>0 {
      printf "%5d  %s/%s  %-6s %s\n", ($4-$3), $3, $4, ($6==""?"-":$6), $1 }' \
    | sort -n | head -n "$limit"
  echo ""
  echo "Tip: REF (screenshot) failures are harder to debug than ATT. Inspect a target with:"
  echo "  $0 errors <test-substring>"
}

cmd_errors() {
  local substr="${1:?usage: $0 errors <test-substring>}"
  [ -f "$REPORT_JSON" ] || { echo "No report. Run: $0 run" >&2; exit 1; }
  python3 - "$substr" "$REPORT_JSON" <<'PY'
import json, sys
substr, path = sys.argv[1], sys.argv[2]
rep = json.load(open(path))
hits = [t for t in rep["results"] if substr in t["test"]]
if not hits:
    print(f"No test matching '{substr}'. (Was it in the suites you ran?)"); sys.exit(0)
for t in hits:
    subs = t.get("subtests") or []
    p = sum(1 for s in subs if s["status"]=="PASS")
    print(f"\n{t['test']}  [{t['status']}]  {p}/{len(subs)} subtests")
    for s in subs:
        if s["status"] != "PASS" and s.get("message"):
            print(f"  FAIL {s['name']}: {s['message']}")
PY
}

cmd_snapshot() {
  local label="${1:?usage: $0 snapshot <label>}"
  parse_results | awk -F'\t' '{print $1"\t"$3"/"$4}' > "$TRIAGE_DIR/snap-$label.txt"
  echo "snapshot '$label' -> $OUT_DIR/snap-$label.txt ($(wc -l < "$TRIAGE_DIR/snap-$label.txt") tests)"
}

cmd_compare() {
  local a="${1:?usage: $0 compare <before> <after>}" b="${2:?usage: $0 compare <before> <after>}"
  local fa="$TRIAGE_DIR/snap-$a.txt" fb="$TRIAGE_DIR/snap-$b.txt"
  [ -f "$fa" ] && [ -f "$fb" ] || { echo "Missing snapshot(s). Have: $(ls "$TRIAGE_DIR"/snap-*.txt 2>/dev/null)" >&2; exit 1; }
  join -t $'\t' <(sort "$fa") <(sort "$fb") | awk -F'\t' '
    { split($2,x,"/"); split($3,y,"/");
      bf=(x[2]>0?x[1]/x[2]:0); af=(y[2]>0?y[1]/y[2]:0);
      if (af>bf) { imp[$1]=$2" -> "$3; ni++ } else if (af<bf) { reg[$1]=$2" -> "$3; nr++ } }
    END {
      printf "\n=== %s -> %s :  %d improved,  %d regressed ===\n", "'"$a"'", "'"$b"'", ni+0, nr+0;
      if (nr) { print "\n--- REGRESSIONS ---"; for (k in reg) print "  "k"  "reg[k] }
      if (ni) { print "\n--- IMPROVEMENTS ---"; for (k in imp) print "  "k"  "imp[k] }
    }'
}

usage() {
  sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

case "${1:-}" in
  run)      shift; cmd_run "$@" ;;
  targets)  shift; cmd_targets "$@" ;;
  errors)   shift; cmd_errors "$@" ;;
  snapshot) shift; cmd_snapshot "$@" ;;
  compare)  shift; cmd_compare "$@" ;;
  ""|-h|--help|help) usage ;;
  *) echo "Unknown command: $1" >&2; usage; exit 1 ;;
esac
