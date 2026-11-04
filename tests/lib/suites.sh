# shellcheck shell=bash
# tests/all.sh's suite runner and its summary of its suites. Each suite has its own summary
# format; a parser below reads one suite's stdout and stderr files and
# prints its counts, `passed failed skipped`, then one line per failing test
# (`name — first line of its reason`). With no summary to find it prints
# nothing and returns 1. Sourced by tests/all.sh; tests/all_summary.rs
# checks it. /usr/bin/awk is mawk here: no gensub, no {n} intervals.

# cargo test: a `test result:` line per test binary, added up (`ignored`
# counts as skipped). A failing test is `test <name> ... FAILED`; its reason
# is the first line of its panic, under `---- <name> stdout ----`.
summary_cargo() {
    cat -- "$@" | awk '
        /^test result: / {
            found = 1
            for (i = 1; i < NF; i++) {
                if ($(i+1) ~ /^passed;/)  p += $i
                if ($(i+1) ~ /^failed;/)  f += $i
                if ($(i+1) ~ /^ignored;/) s += $i
            }
        }
        /^test .* \.\.\. FAILED$/ { if (!($2 in why)) { order[++n] = $2; why[$2] = "" } }
        /^---- .* stdout ----$/ { cur = $2 }
        cur != "" && / panicked at / {
            line = $0
            if (line ~ /:$/) getline line
            else sub(/.* panicked at /, "", line)
            why[cur] = line; cur = ""
        }
        END {
            if (!found) exit 1
            print p + 0, f + 0, s + 0
            for (i = 1; i <= n; i++) print order[i] (why[order[i]] != "" ? " — " why[order[i]] : "")
        }'
}

# Python unittest: `Ran N tests`, then `OK` or `FAILED`, either with
# `(failures=F, errors=E, skipped=K, ...)`. A failing test is a `FAIL:` or
# `ERROR:` header; its reason is the unindented exception line its
# traceback ends on.
summary_unittest() {
    cat -- "$@" | awk '
        /^Ran [0-9]+ tests? in / { ran = $2; seen_ran = 1 }
        seen_ran && /^(OK|FAILED)( \(.*\))?$/ {
            verdict = 1
            line = $0; sub(/^[A-Z]+ ?\(?/, "", line); sub(/\)$/, "", line)
            k = split(line, parts, ", ")
            for (i = 1; i <= k; i++) { split(parts[i], kv, "="); c[kv[1]] = kv[2] + 0 }
        }
        /^(FAIL|ERROR): / { cur = $0; sub(/^[A-Z]+: /, "", cur); order[++n] = cur; tb = 0 }
        cur != "" && /^Traceback / { tb = 1; next }
        tb && /^[^ \t]/ { why[cur] = $0; tb = 0; cur = "" }
        END {
            if (!verdict) exit 1
            f = c["failures"] + c["errors"] + c["unexpected successes"]
            s = c["skipped"] + 0
            print ran - f - s, f, s
            for (i = 1; i <= n; i++) print order[i] (why[order[i]] != "" ? " — " why[order[i]] : "")
        }'
}

# bun, as claude plugin test runs it: ` N pass`, ` N fail` and ` N skip`.
# A failing test is `(fail) <name>`, maybe with ` [time]`; its reason is the
# `error:` line printed above it. bun lists failures again at the foot, so
# each is named once.
summary_bun() {
    cat -- "$@" | awk '
        /^ *[0-9]+ pass$/ { p = $1; seen_p = 1 }
        /^ *[0-9]+ fail$/ { f = $1; seen_f = 1 }
        /^ *[0-9]+ skip$/ { s = $1 }
        /^error: / { err = $0; sub(/^error: /, "", err) }
        /^\(fail\) / {
            name = $0; sub(/^\(fail\) /, "", name); sub(/ \[[0-9.]+m?s\]$/, "", name)
            if (!(name in why)) { order[++n] = name; why[name] = err }
            err = ""
        }
        END {
            if (!(seen_p && seen_f)) exit 1
            print p + 0, f + 0, s + 0
            for (i = 1; i <= n; i++) print order[i] (why[order[i]] != "" ? " — " why[order[i]] : "")
        }'
}

# The e2e scripts: `passed: N   failed: N   skipped: N` (e2e-tag.sh's had no
# skipped field before 2026-11, so it reads as 0). A failing check is its
# `  FAIL  ` line, which carries its own reason.
summary_e2e() {
    cat -- "$@" | awk '
        /^passed: [0-9]+ +failed: [0-9]+/ { found = 1; p = $2; f = $4; s = ($5 == "skipped:") ? $6 : 0 }
        /^  FAIL  / { order[++n] = substr($0, 9) }
        END {
            if (!found) exit 1
            print p + 0, f + 0, s + 0
            for (i = 1; i <= n; i++) print order[i]
        }'
}

# The runner. Each suite's stdout and stderr go to the terminal as before
# and to files of their own here, for its parser and its last stderr line.
SUITE_LOGS=$(mktemp -d) || exit 1
trap 'rm -rf "$SUITE_LOGS"' EXIT
results=()      # the closing block's lines, in the order the suites ran
not_counted=()  # suites whose counts could not be read
suites_total=0 suites_passed=0 suites_failed=0 suites_skipped=0
tests_passed=0 tests_failed=0 tests_skipped=0

suite() {  # name, reason it cannot run (empty if it can), summary format, command...
    local name="$1" why="$2" format="$3"; shift 3
    suites_total=$((suites_total + 1))
    if [ -n "$why" ]; then
        printf '\n\033[1m── %s ──\033[0m  SKIP: %s\n' "$name" "$why"
        results+=("  SKIP  $name — $why")
        suites_skipped=$((suites_skipped + 1))
        return
    fi
    printf '\n\033[1m── %s ──\033[0m\n' "$name"
    # The command as run, so that a reader of the log (a branch report)
    # can name it rather than guess it.
    printf '$ %s\n' "$*"
    local start=$SECONDS out="$SUITE_LOGS/$suites_total.out" err="$SUITE_LOGS/$suites_total.err"
    # stderr through one tee, stdout through the other; the subshell exits
    # with the suite's own status, so PIPESTATUS[0] is the suite's, never tee's.
    ( "$@" 2>&1 1>&3 3>&- | tee "$err" >&2; exit "${PIPESTATUS[0]}" ) 3>&1 | tee "$out"
    local rc=${PIPESTATUS[0]}
    local secs=$((SECONDS - start)) parsed p f s last word failing line
    parsed=$(summary_"$format" "$out" "$err" 2>/dev/null)
    read -r p f s <<<"${parsed%%$'\n'*}"
    last=$(grep -v '^[[:space:]]*$' "$err" | tail -n 1)
    if [[ "$p $f $s" =~ ^[0-9]+\ [0-9]+\ [0-9]+$ ]]; then
        tests_passed=$((tests_passed + p))
        tests_failed=$((tests_failed + f))
        tests_skipped=$((tests_skipped + s))
        word=PASS
        if [ "$rc" -ne 0 ] || [ "$f" -gt 0 ]; then word=FAIL; fi
        results+=("  $word  $name — $p passed, $f failed, $s skipped (${secs}s)")
        failing=$(tail -n +2 <<<"$parsed")
        if [ -n "$failing" ]; then
            # "      - ": a failing test's line never starts FAIL, PASS or ──,
            # so a reader can't take it for a suite's line or the rule.
            while IFS= read -r line; do results+=("      - $line"); done <<<"$failing"
        elif [ "$word" = FAIL ]; then
            results+=("      - exited $rc, naming no failing test: ${last:-nothing on stderr}")
        fi
    elif [ "$rc" -ne 0 ]; then
        word=FAIL
        results+=("  FAIL  $name — stopped before its summary: ${last:-nothing on stderr} (${secs}s)")
        not_counted+=("$name")
    else
        # Exit 0 and no summary read: a parser out of date, not a red suite.
        word=PASS
        results+=("  PASS  $name — counts unreadable (${secs}s)")
        not_counted+=("$name")
    fi
    if [ "$word" = FAIL ]; then
        suites_failed=$((suites_failed + 1))
    else
        suites_passed=$((suites_passed + 1))
    fi
}

# One line per suite, in run order, failing tests under their suite, then
# suites and tests counted on one line worded so neither reads as the other.
# A skip is not a failure: only a suite that ran and said no fails this.
closing_block() {
    local uncounted=""
    if [ "${#not_counted[@]}" -gt 0 ]; then
        printf -v uncounted '%s, ' "${not_counted[@]}"
        uncounted=" (not counted: ${uncounted%, })"
    fi
    echo
    echo "──────────────────────────────────────────────────────────────"
    printf '%s\n' "${results[@]}"
    echo
    echo "suites: $suites_passed of $suites_total passed, $suites_failed failed, $suites_skipped skipped · tests: $tests_passed passed, $tests_failed failed, $tests_skipped skipped$uncounted"
    [ "$suites_failed" -eq 0 ]
}
