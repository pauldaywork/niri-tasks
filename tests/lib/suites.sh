# shellcheck shell=bash
# tests/all.sh's summary of its suites. Each suite has its own summary
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
