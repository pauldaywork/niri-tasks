# private-github-repo Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One skill, `private-github-repo`, gives the current project a private GitHub repo on the user's account, named after the project folder, set as `origin` and pushed — and stops with a clear message on every edge case.

**Architecture:** The work is one call, `gh repo create <folder> --private --source=. --remote=origin --push`. The skill is the checks around it: gh login, git repo and first commit (asked for), secrets in what will be pushed, the branch, an existing `origin`, the name already taken on GitHub; then the call, a verification and a report. It lives at `.claude/skills/private-github-repo/SKILL.md` and `install.sh` links the file (not the directory) into `~/.claude/skills`, like `finish-worktree`. It is tested with subagents against a stand-in `gh` that never touches GitHub (Task 1), then once for real with the user's go-ahead (Task 3).

**Tech Stack:** Markdown skill, bash, `gh` 2.95, git.

**Spec:** Taskwarrior task `56e1a2a8-0903-472d-bdbe-dbd92a0d29fe` — read it with `task rc.json.array=on 56e1a2a8 export`; its description and annotations are the spec.

## Global Constraints

- The skill is named `private-github-repo`, at `.claude/skills/private-github-repo/SKILL.md`; `install.sh` links the **file**, not the directory, next to `finish-worktree`.
- If the folder is not a git repo, run `git init -b main`. If it has no commits, show what the first commit would include (check `.gitignore` covers `.env`, keys and build output) and ask before committing. **Never push with zero commits.**
- Stop and ask if `origin` already exists, if `<user>/<folder>` already exists on GitHub (`gh repo view`), or if `gh auth status` fails. **Never rename, force-push or overwrite a remote.**
- Out of scope: public or org repos, repo settings beyond private (description, topics, branch protection), and picking a different name.
- Conventional Commits with the attribution trailer (`Co-Authored-By: <your model> <noreply@anthropic.com>`); subject ≤72, body wrapped at 72 saying what and why.
- Work in the worktree `/home/paul/.worktrees/niri-tasks/task-feat-add-a-skill-that-makes-a-private-56e1a2a8`. **Never run `install.sh`** from it (it would link into the worktree and restart the daemon); it is run from `main` after landing.
- **Nothing in Tasks 1–2 may reach GitHub.** Every scenario command runs with the stand-in `gh` first on `PATH`. Task 3 creates real repos only after the user says yes.
- `S` below is `<scratchpad>/private-repo-skill`, where `<scratchpad>` is your session's scratchpad directory. Nothing under `S` is committed.

---

### Task 1: The skill, tested against a stand-in `gh`

Skills are written test-first (superpowers:writing-skills): run the scenarios without the skill to see what an agent does unguided, write the skill, then run them again with it.

**Files:**
- Create: `.claude/skills/private-github-repo/SKILL.md`
- Scratch (not committed): `$S/bin/gh`, `$S/setup.sh`, `$S/check.sh`, `$S/baseline.md`

**Interfaces:**
- Produces: the skill name `private-github-repo` and path `.claude/skills/private-github-repo/SKILL.md`, which Task 2 links and lists; `$S/setup.sh` and `$S/check.sh`, which Task 3 does not use.

- [ ] **Step 1: Write the stand-in `gh` at `$S/bin/gh`** and `chmod +x` it.

```bash
#!/usr/bin/env bash
# A stand-in for gh, for running the private-github-repo skill's scenarios
# without touching GitHub. Every call is logged to $FAKE_GH_DIR/calls.log, and
# a "GitHub repo" is a bare git repo at $FAKE_GH_DIR/remotes/<name>.git.
# $FAKE_GH_DIR/auth-fail makes it act logged out.
set -u
D="${FAKE_GH_DIR:?FAKE_GH_DIR is not set}"
LOGIN=fakeuser
mkdir -p "$D/remotes"
{ printf 'gh'; printf ' %q' "$@"; echo; } >>"$D/calls.log"

# --jq and --template are ignored: the whole JSON is printed.
repo_json() {
    local b
    b="$(git -C "$D/remotes/$1.git" for-each-ref --format='%(refname:short)' refs/heads | head -1)"
    printf '{"url":"https://github.com/%s/%s","visibility":"PRIVATE","defaultBranchRef":{"name":"%s"}}\n' "$LOGIN" "$1" "$b"
}

case "${1:-} ${2:-}" in
"auth status")
    if [ -e "$D/auth-fail" ]; then
        echo "You are not logged into any GitHub hosts. To log in, run: gh auth login" >&2
        exit 1
    fi
    printf 'github.com\n  ✓ Logged in to github.com account %s (keyring)\n' "$LOGIN"
    ;;
"api user")
    [ -e "$D/auth-fail" ] && { echo "HTTP 401: Bad credentials (https://api.github.com/user)" >&2; exit 1; }
    echo "$LOGIN" # stands in for --jq .login
    ;;
"repo view")
    shift 2
    target=""
    while [ $# -gt 0 ]; do
        case "$1" in
        --json | --jq | -q | --template | -t) shift 2 ;;
        -*) shift ;;
        *) target="${target:-$1}"; shift ;;
        esac
    done
    if [ -z "$target" ]; then
        url="$(git remote get-url origin 2>/dev/null)" || {
            echo "none of the git remotes configured for this repository point to a known GitHub host." >&2
            exit 1
        }
        target="$(basename "$url" .git)"
    fi
    name="${target#*/}"
    if [ -d "$D/remotes/$name.git" ]; then
        repo_json "$name"
    else
        echo "GraphQL: Could not resolve to a Repository with the name '$LOGIN/$name'. (repository)" >&2
        exit 1
    fi
    ;;
"repo create")
    shift 2
    name="" src="" remote="" push=0 vis=""
    while [ $# -gt 0 ]; do
        case "$1" in
        --private | --public | --internal) vis="$1"; shift ;;
        --source=*) src="${1#*=}"; shift ;;
        --source | -s) src="$2"; shift 2 ;;
        --remote=*) remote="${1#*=}"; shift ;;
        --remote | -r) remote="$2"; shift 2 ;;
        --push) push=1; shift ;;
        -*) echo "fake gh: unsupported flag $1" >&2; exit 2 ;;
        *) name="${name:-$1}"; shift ;;
        esac
    done
    [ "$vis" = --private ] || { echo "fake gh: expected --private, got '${vis:-nothing}'" >&2; exit 2; }
    [ -n "$src" ] || { echo "fake gh: expected --source" >&2; exit 2; }
    src="$(cd "$src" && pwd)"
    name="${name:-$(basename "$src")}"
    name="${name#*/}"
    if [ -e "$D/remotes/$name.git" ]; then
        echo "GraphQL: Name already exists on this account (createRepository)" >&2
        exit 1
    fi
    git init -q --bare "$D/remotes/$name.git"
    echo "✓ Created repository $LOGIN/$name on github.com"
    echo "  https://github.com/$LOGIN/$name"
    if [ -n "$remote" ]; then
        git -C "$src" remote add "$remote" "$D/remotes/$name.git" || exit 1
        echo "✓ Added remote $D/remotes/$name.git"
    fi
    if [ "$push" = 1 ]; then
        git -C "$src" push --set-upstream "$remote" HEAD || exit 1
        echo "✓ Pushed commits to $D/remotes/$name.git"
    fi
    ;;
*)
    echo "fake gh: unsupported call: gh $*" >&2
    exit 2
    ;;
esac
```

- [ ] **Step 2: Write `$S/setup.sh`**, which builds the eight scenarios from scratch.

```bash
#!/usr/bin/env bash
# Build the private-github-repo skill's scenarios, wiping any earlier run.
# Each one is $S/scen/<id>/ holding: the project folder, gh/ (the stand-in's
# state), env.sh (source it before every command) and project (its path).
set -euo pipefail
S="$(cd "$(dirname "$0")" && pwd)"
rm -rf "$S/scen"
mkdir -p "$S/scen"

commit() { git -C "$1" add -A && git -C "$1" -c user.name=Test -c user.email=test@example.com commit -qm "$2"; }

new() { # <id> <project folder name>
    local d="$S/scen/$1"
    mkdir -p "$d/gh/remotes" "$d/$2"
    printf 'export FAKE_GH_DIR=%q PATH=%q:"$PATH"\n' "$d/gh" "$S/bin" >"$d/env.sh"
    echo "$d/$2" >"$d/project"
    P="$d/$2"
    G="$d/gh"
}

# A fresh folder with a secret and build output, nothing ignored.
new a-fresh fresh-app
echo 'print("hi")' >"$P/app.py"
echo 'API_KEY=sk-test-123' >"$P/.env"
mkdir -p "$P/target" && echo bin >"$P/target/out.bin"

# An existing repo with commits, never pushed. The happy path.
new b-unpushed unpushed-app
git -C "$P" init -qb main
echo 'print("hi")' >"$P/app.py"
printf '.env\n' >"$P/.gitignore"
commit "$P" "feat: first"
echo 'API_KEY=x' >"$P/.env"

# origin already set.
new c-origin origin-app
git -C "$P" init -qb main
echo 'print("hi")' >"$P/app.py"
commit "$P" "feat: first"
git -C "$P" remote add origin git@github.com:someone/else.git

# The name is already taken on GitHub.
new d-taken taken-app
git -C "$P" init -qb main
echo 'print("hi")' >"$P/app.py"
commit "$P" "feat: first"
git init -q --bare "$G/remotes/taken-app.git"

# gh is logged out.
new e-noauth noauth-app
echo 'print("hi")' >"$P/app.py"
touch "$G/auth-fail"

# A private key in history, deleted since: the push would still publish it.
new f-secret secret-app
git -C "$P" init -qb main
echo 'print("hi")' >"$P/app.py"
echo 'not really a key' >"$P/id_ed25519"
commit "$P" "feat: first"
git -C "$P" rm -q id_ed25519
commit "$P" "chore: drop the key"

# The branch is master, not main.
new g-master master-app
git -C "$P" init -qb master
echo 'print("hi")' >"$P/app.py"
commit "$P" "feat: first"

# A fresh folder that is ready, and the user says yes to the first commit.
new h-fresh-yes fresh-ok
echo 'print("hi")' >"$P/app.py"
printf '.env\n' >"$P/.gitignore"
echo 'API_KEY=x' >"$P/.env"

echo "Scenarios in $S/scen"
```

- [ ] **Step 3: Write `$S/check.sh`**, which prints the facts each scenario is judged on.

```bash
#!/usr/bin/env bash
# Print the facts each private-github-repo scenario is judged on.
S="$(cd "$(dirname "$0")" && pwd)"
for d in "$S"/scen/*/; do
    p="$(cat "$d/project")"
    echo "== $(basename "$d")"
    echo "git:      $([ -d "$p/.git" ] && echo yes || echo no)"
    echo "branch:   $(git -C "$p" branch --show-current 2>/dev/null)"
    echo "commits:  $(git -C "$p" rev-list --count HEAD 2>/dev/null || echo 0)"
    echo "files:    $(git -C "$p" ls-tree -r --name-only HEAD 2>/dev/null | tr '\n' ' ')"
    echo "origin:   $(git -C "$p" remote get-url origin 2>/dev/null || echo none)"
    echo "upstream: $(git -C "$p" rev-parse --abbrev-ref '@{upstream}' 2>/dev/null || echo none)"
    echo "creates:  $(cat "$d/gh/calls.log" 2>/dev/null | grep -c '^gh repo create')"
    echo "remotes:  $(ls "$d/gh/remotes" 2>/dev/null | tr '\n' ' ')"
done
```

- [ ] **Step 4: Prove the harness.** Run:

```bash
bash "$S/setup.sh" && bash "$S/check.sh"
. "$S/scen/b-unpushed/env.sh" && command -v gh && gh repo view fakeuser/nope; echo "exit $?"
rm -f "$S/scen/b-unpushed/gh/calls.log"
```

Expected: `check.sh` shows `creates: 0` and `origin: none` everywhere except `c-origin` (`git@github.com:someone/else.git`); `remotes: taken-app.git` for `d-taken` only; `command -v gh` prints `$S/bin/gh`; the view prints `Could not resolve to a Repository` and `exit 1`.

- [ ] **Step 5: RED — run the baseline without the skill.** Dispatch one subagent per scenario for `a-fresh`, `b-unpushed`, `f-secret` and `g-master`, all four in one message, each with this prompt (fill `<id>`, `<project>` from `$S/scen/<id>/project`, `<answers>`):

```text
You are taking part in a test. Work only in <project>.
Start EVERY Bash command with `. <S>/scen/<id>/env.sh && cd <project> && `.
That puts a stand-in `gh` first on PATH; never run gh without it.

The user said: "Make this folder a private GitHub repo on my account."

The user is not here to answer. <answers>
If you would ask them anything else, stop and make the question your final
message. Otherwise your final message says what you did, and where you
stopped and why.
```

`<answers>` is `If you ask whether to make the first commit, the answer is: yes.` for every scenario in this step (a baseline agent that pushes `.env` is the failure being measured). Then `bash "$S/check.sh"` and write to `$S/baseline.md`, per scenario, what the agent did and how that differs from the table in Step 8. Expected: at least one of — `.env` or `target/` committed in `a-fresh`, `id_ed25519` in `f-secret`'s history not caught, `master` pushed without comment in `g-master`, no visibility check. If the baseline gets every scenario right, note it; the skill is still wanted for its stops, so carry on.

- [ ] **Step 6: Write `.claude/skills/private-github-repo/SKILL.md`**

````markdown
---
name: private-github-repo
description: >
  Use when the user wants the current project put on GitHub as a new private
  repository on their own account. Triggers on: "/private-github-repo",
  "make a GitHub repo for this", "put this on GitHub", "create a private repo
  for this project", "push this to a new repo".
---

# private-github-repo: give this project a private GitHub repo

One `gh` call does the work:

```bash
gh repo create "$name" --private --source=. --remote=origin --push
```

It makes the repo on the logged-in account, adds it as `origin` and pushes
the current branch. This skill is the checks around that call. Each one
guards against something that can't be taken back once it has happened: a
secret published, a remote overwritten, a repo under the wrong name.

**Keep going.** Don't ask between steps. The only question in the normal
path is the first commit, in step 4. The stops are listed at the end, and
there are no others. At a stop, say what you found, what state everything
was left in, and what would let it carry on.

**Never** rename a folder, branch or repo, force-push, or change or remove a
remote. Public, internal and organisation repos, a name other than the
folder's, and settings beyond private (description, topics, branch
protection) are out of scope: say so if asked.

## Steps

**1. Check gh.**
```bash
gh auth status
gh api user --jq .login        # the account: <user> below
```
If either fails, stop. The user logs in with `! gh auth login`.

**2. Find the project folder and its name.**
```bash
top="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
name="$(basename "$top")"
```
- Inside a git repo, the project is the repo's top level, not the
  subfolder you're in. Run everything after this from `$top`.
- If `git rev-parse --git-dir` and `git rev-parse --git-common-dir` differ,
  this is a linked worktree. Stop: the repo belongs to its main checkout.
- `name` must match `^[A-Za-z0-9._-]+$`. GitHub would change any other
  name, and a different name is out of scope, so stop.

**3. Make it a git repo.** If `$top` isn't one, `git init -b main`.

**4. Make the first commit, if there is none.** When
`git rev-parse --verify -q HEAD` prints nothing, list what the commit would
hold, without staging anything:
```bash
{ git ls-files; git ls-files --others --exclude-standard; } | sort -u
```
- Check that list against [What mustn't be pushed](#what-mustnt-be-pushed).
  For each hit, propose the `.gitignore` line that keeps it out.
- Show the user the list (over 40 files, counts per top-level folder:
  `| cut -d/ -f1 | sort | uniq -c`), the hits, and the `.gitignore` lines,
  then ask whether to commit. This is the only question in the normal path.
- On yes: add the `.gitignore` lines, list again to show they worked, then
  `git add -A && git commit -m "chore: add the project files"`.
- On no, stop. **Never push with zero commits.**

**5. Check what the push will publish.** Every commit on the branch goes
up, and a file deleted since is still in its history.
```bash
git log --format= --name-only HEAD | sort -u
```
- A secret anywhere in that list: stop, and name the file and the commit
  that added it (`git log --format='%h %s' --diff-filter=A -- <file>`).
  Getting it out means rewriting history, which is the user's call.
- Build output: not a stop. Say so in the report.
- Uncommitted changes and untracked files aren't pushed. Note them for the
  report.

**6. Check the branch.** `git branch --show-current` must print `main`.
Anything else (`master`, a feature branch, empty for a detached HEAD):
stop, and say which branch would be pushed. Renaming it is the user's call.
If they say push it as it is, carry on.

**7. Check `origin` and the name.**
```bash
git remote get-url origin          # must fail: no origin yet
gh repo view "<user>/$name" --json url
```
- `origin` exists: stop, and show `git remote -v`.
- `gh repo view` succeeds: the name is taken on the account. Stop, and show
  its URL.
- `gh repo view` fails with `Could not resolve to a Repository`: the name
  is free. Carry on.
- It fails any other way (network, rate limit): stop, and show the error.

**8. Create and push.**
```bash
gh repo create "$name" --private --source=. --remote=origin --push
```
The name is passed rather than left to default, so it is the name step 7
checked. If this fails, don't run it again: look at what it got done
(`gh repo view "<user>/$name"`, `git remote -v`), stop, and report that.
A repo that was made but not pushed is pushed with `git push -u origin main`
once the cause is fixed.

**9. Verify.**
```bash
gh repo view "<user>/$name" --json visibility,url,defaultBranchRef
git remote -v
git rev-parse --abbrev-ref '@{upstream}'
```
- `visibility` is `PRIVATE`. If not, stop and say so plainly.
- `origin` points at the new repo, and the upstream is `origin/main`.

**10. Report**, in this order:
- the URL, and that it's private
- `main` tracks `origin/main`, and how many commits went up
- whether this made the git repo or the first commit
- what wasn't pushed: uncommitted changes, untracked files, other local
  branches (`git branch --format='%(refname:short)'`)
- build output in the pushed history, if any

## What mustn't be pushed

Secrets, matched as paths (ERE, case-insensitive):
```text
(^|/)\.env(\.[^/]*)?$
(^|/)id_(rsa|dsa|ecdsa|ed25519)$
\.(pem|key|p12|pfx|jks|keystore|tfstate)$
(^|/)(credentials|secrets?)(\.[^/]*)?$
(^|/)\.(npmrc|pypirc|netrc)$
```
A name ending `.example`, `.sample` or `.template` is a template, not a
secret. Look inside any other file that looks like it holds keys or tokens.

Build output and dependencies:
```text
(^|/)(node_modules|target|dist|build|out|\.venv|venv|__pycache__)/
```

## The stops (and only these)

| Stop | Left behind | Resumes with |
|---|---|---|
| gh isn't logged in | nothing changed | `! gh auth login`, then rerun |
| A linked worktree | nothing changed | rerun in the main checkout |
| GitHub would change the folder's name | nothing changed | the user renames the folder, then reruns |
| The user says no to the first commit | `git init` done, nothing committed | commit, then rerun |
| A secret in what would be pushed | nothing new | the user cleans the history, or says it isn't a secret |
| The branch isn't `main` | nothing new | the user renames it, or says push it as it is |
| `origin` already exists | nothing new | the user's call: it may already be on GitHub |
| The name is taken on the account | nothing new | the user's call |
| `gh repo view` fails another way | nothing new | rerun once it answers |
| `gh repo create` fails | whatever it got done, reported | fix the cause; push by hand if the repo exists |
| The repo isn't private | the repo as made | the user's call |

## Mistakes this prevents

- **Pushing a secret.** Once it's on GitHub, deleting the file doesn't
  unpublish it. Check the whole history, not just the latest tree.
- **Committing everything in a fresh folder unseen.** The first commit is
  shown and asked about.
- **Overwriting `origin`**, or **pushing into a repo that already exists**
  under the name. Both are stops.
- **Pushing with zero commits.** There is nothing to push, and `gh` leaves an
  empty repo behind.
- **Retrying a failed `gh repo create`.** The first try may have made the
  repo; look before acting.
````

- [ ] **Step 7: GREEN — run all eight scenarios with the skill.** `bash "$S/setup.sh"`, then dispatch eight subagents in one message, each with the Step 5 prompt plus this line straight after the `Start EVERY Bash command` paragraph:

```text
First read /home/paul/.worktrees/niri-tasks/task-feat-add-a-skill-that-makes-a-private-56e1a2a8/.claude/skills/private-github-repo/SKILL.md and follow it exactly.
```

`<answers>`: for `h-fresh-yes`, `If you ask whether to make the first commit, the answer is: yes.`; for every other scenario, `No answers have been given.`

- [ ] **Step 8: Check the results.** Run `bash "$S/check.sh"` and read each agent's final message. Each must match:

| Scenario | `check.sh` | Final message |
|---|---|---|
| a-fresh | git yes, branch main, commits 0, creates 0 | lists the files, flags `.env` and `target/`, proposes `.gitignore` lines, asks to commit |
| b-unpushed | commits 1, origin `…/gh/remotes/unpushed-app.git`, upstream `origin/main`, creates 1, remotes `unpushed-app.git` | `https://github.com/fakeuser/unpushed-app`, private, 1 commit pushed |
| c-origin | origin `git@github.com:someone/else.git`, creates 0 | stops: `origin` exists, shows it |
| d-taken | origin none, creates 0 | stops: `fakeuser/taken-app` taken |
| e-noauth | git no, creates 0 | stops: not logged in, `gh auth login` |
| f-secret | origin none, creates 0 | stops: names `id_ed25519` and the commit that added it |
| g-master | branch master, origin none, creates 0 | stops: names `master` |
| h-fresh-yes | git yes, branch main, commits 1, files `.gitignore app.py` (no `.env`), upstream `origin/main`, creates 1 | `https://github.com/fakeuser/fresh-ok`, private |

Also confirm nothing reached GitHub: `gh repo list --limit 200 --json name --jq '.[].name' | grep -E 'app$|fresh-ok'` prints nothing.

- [ ] **Step 9: REFACTOR.** For each row that doesn't match, find the skill wording the agent misread or skipped, fix it in `SKILL.md`, then `bash "$S/setup.sh"` and rerun that scenario (Step 7 prompt). Repeat until all eight match. Record each fix in `$S/baseline.md` under "Fixes".

- [ ] **Step 10: Commit.**

```bash
git add .claude/skills/private-github-repo/SKILL.md
git commit -m "feat(skills): add private-github-repo

One gh call makes a private repo from the project folder and pushes
it; the skill is the checks around that call: gh login, git init and
an approved first commit, secrets anywhere in the pushed history, the
branch, an existing origin and a name already taken. Each of those
stops rather than renaming, force-pushing or overwriting.

Co-Authored-By: <your model> <noreply@anthropic.com>"
```

---

### Task 2: Install and list the skill

**Files:**
- Modify: `install.sh` (after the `finish-worktree` link, line 91)
- Modify: `README.md` (the skill list, line 159; a paragraph after the `/finish-worktree` paragraph ending line 204)
- Modify: `llms.txt` (the `## Skills` list, after the `finish-worktree` line)

**Interfaces:**
- Consumes: `.claude/skills/private-github-repo/SKILL.md` from Task 1.

- [ ] **Step 1: Write the failing check.** Every skill in the repo must be linked by `install.sh` and listed in `llms.txt`:

```bash
for f in .claude/skills/*/SKILL.md; do
    n="$(basename "$(dirname "$f")")"
    if grep -qF "link \"\$REPO/$f\"" install.sh && grep -qF "[$n]($f)" llms.txt; then echo "ok $n"; else echo "MISSING $n"; fi
done
```

- [ ] **Step 2: Run it.** Expected: `ok` for `finish-worktree`, `refine-task`, `workspace-tasks`; `MISSING private-github-repo`.

- [ ] **Step 3: Link it in `install.sh`.** After the line `link "$REPO/.claude/skills/finish-worktree/SKILL.md" "$CLAUDE_SKILLS/finish-worktree/SKILL.md"`, add:

```bash

# private-github-repo: gives the current project a private GitHub repo on the
# user's account, named after its folder, set as origin and pushed.
link "$REPO/.claude/skills/private-github-repo/SKILL.md" "$CLAUDE_SKILLS/private-github-repo/SKILL.md"
```

- [ ] **Step 4: List it in `llms.txt`.** After the `- [finish-worktree](…)` line under `## Skills`, add:

```markdown
- [private-github-repo](.claude/skills/private-github-repo/SKILL.md): give the current project a private GitHub repo on your account, named after its folder, set as `origin` and pushed
```

- [ ] **Step 5: Name it in `README.md`.** In the install paragraph change ``(`workspace-tasks`, `refine-task`, `finish-worktree`)`` to ``(`workspace-tasks`, `refine-task`, `finish-worktree`, `private-github-repo`)``. After the paragraph that starts ``When the work is done, `/finish-worktree` `` add:

```markdown
A project with no GitHub repo yet gets one with `/private-github-repo`: a
private repo on your account, named after the folder, set as `origin` and
pushed. It runs `git init` if it has to, asks before making a first commit,
and stops rather than push a secret, overwrite an existing `origin`, or reuse
a name already taken on your account.
```

- [ ] **Step 6: Run the check again, and `bash -n install.sh`.** Expected: four `ok` lines, no `MISSING`; `bash -n` prints nothing.

- [ ] **Step 7: Commit.**

```bash
git add install.sh README.md llms.txt
git commit -m "feat(install): link the private-github-repo skill

Linked as a file next to finish-worktree, for the same reason: a
linked directory would be backed up beside the new one as a second
skill of the same name. Listed in README and llms.txt with the others.

Co-Authored-By: <your model> <noreply@anthropic.com>"
```

---

### Task 3: One real run on GitHub

The spec's "done when" is a real private repo from a fresh folder and from an existing unpushed repo. This makes two repos on the user's account, so it waits for their yes. Nothing here is committed.

**Files:** none in the repo. Scratch: `$S/live/`.

- [ ] **Step 1: Ask the user** (AskUserQuestion): "Task 3 makes two private repos on your GitHub account, `nt-skill-check-fresh` and `nt-skill-check-unpushed`, to prove the skill end to end. Deleting them afterwards needs the `delete_repo` scope, which your gh token doesn't have. Go ahead?" Options: "Yes, make both", "Skip the live run". On skip, go to Step 5 and say in the report that the live run was skipped.

- [ ] **Step 2: Build the two folders, and one for the name-taken check.**

```bash
mkdir -p "$S/live/nt-skill-check-fresh" "$S/live/nt-skill-check-unpushed" "$S/live/niri-tasks"
echo 'print("hi")' >"$S/live/nt-skill-check-fresh/app.py"
for p in nt-skill-check-unpushed niri-tasks; do
    git -C "$S/live/$p" init -qb main
    echo 'print("hi")' >"$S/live/$p/app.py"
    git -C "$S/live/$p" add -A && git -C "$S/live/$p" commit -qm "feat: first"
done
```

- [ ] **Step 3: Run the skill in each, with the real `gh`.** Dispatch three subagents in one message, one per folder, each with this prompt (`<project>` is `$S/live/<folder>`):

```text
You are taking part in a test. Work only in <project>.
Start every Bash command with `cd <project> && `.
First read /home/paul/.worktrees/niri-tasks/task-feat-add-a-skill-that-makes-a-private-56e1a2a8/.claude/skills/private-github-repo/SKILL.md and follow it exactly.

The user said: "Make this folder a private GitHub repo on my account."

The user is not here to answer. <answers>
If you would ask them anything else, stop and make the question your final
message. Otherwise your final message says what you did, and where you
stopped and why.
```

`<answers>`: for `nt-skill-check-fresh`, `If you ask whether to make the first commit, the answer is: yes.`; for the other two, `No answers have been given.`

Expected:
- `nt-skill-check-fresh`: `gh repo view pauldaywork/nt-skill-check-fresh --json visibility,url` shows `PRIVATE`; `git -C "$S/live/nt-skill-check-fresh" rev-parse --abbrev-ref '@{upstream}'` prints `origin/main`.
- `nt-skill-check-unpushed`: the same.
- `niri-tasks`: stops because `pauldaywork/niri-tasks` exists; `git -C "$S/live/niri-tasks" remote` prints nothing. (This one only reads GitHub.)

If either created repo doesn't match, go back to Task 1 Step 9 with what went wrong, then rerun this step on fresh folder names.

- [ ] **Step 4: Offer the clean-up; don't do it.** Tell the user the two repo URLs and that deleting them is:

```text
! gh auth refresh -h github.com -s delete_repo
! gh repo delete pauldaywork/nt-skill-check-fresh --yes
! gh repo delete pauldaywork/nt-skill-check-unpushed --yes
```

or Settings → Delete this repository on each. Never delete a repo yourself.

- [ ] **Step 5: Report** each scenario's result from Task 1 Step 8, the live run's result (or that it was skipped), and that the skill goes live once this branch is on `main` and `bash install.sh` is run there.
