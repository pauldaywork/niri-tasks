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
start="$(pwd)"
top="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
name="$(basename "$top")"
cd "$top"
```
- Shell state doesn't survive between commands. From here on, use the
  literal values of `top`, `name` and `user` (not the variables), and start
  each command with `cd` to the project folder.
- Inside a git repo, the project is the repo's top level, not the
  subfolder you're in. If `top` isn't `start` (a subfolder, or a folder
  inside some other repo such as a dotfiles repo), name both folders when
  you ask in step 4 and in the report. The `cd` is part of the step: later
  commands (`git ls-files`, `git add -A`) assume the working directory is
  the top level, so stay there for the rest of the skill.
- Inside a git repo, if
  `git rev-parse --path-format=absolute --git-dir` and
  `git rev-parse --path-format=absolute --git-common-dir` print different
  paths, this is a linked worktree. Stop: the repo belongs to its main checkout.
  (Both are absolute, so running from a subfolder doesn't make them differ.)
- `name` must match `^[A-Za-z0-9._-]+$`. GitHub would change any other
  name, and a different name is out of scope, so stop.
- Check the name is free, before anything is created:
  ```bash
  gh repo view "<user>/<name>" --json url
  ```
  - It succeeds: the name is taken on the account. Stop, and show its URL.
  - It fails with `Could not resolve to a Repository`: the name is free.
    Carry on.
  - It fails any other way (network, rate limit): stop, and show the error.

**3. Make it a git repo.** If `$top` isn't one, `git init -b main`.

**4. Make the first commit, if there is none.** When
`git rev-parse --verify -q HEAD` prints nothing, list what the commit would
hold, without staging anything:
```bash
{ git -c core.quotePath=false ls-files
  git -c core.quotePath=false ls-files --others --exclude-standard; } | sort -u
```
- Pipe that list through the filter in
  [What mustn't be pushed](#what-mustnt-be-pushed). For each hit, propose
  the `.gitignore` line that keeps it out.
- Show the user the list (over 40 files, counts per top-level folder:
  `| cut -d/ -f1 | sort | uniq -c`), the hits, and the `.gitignore` lines,
  then ask whether to commit. This is the only question in the normal path.
- On yes: add the `.gitignore` lines and list again. A hit that is still
  listed was already staged, and `.gitignore` doesn't unstage it: run
  `git rm --cached <file>` on it and list again. Once the filter prints
  nothing, `git add -A && git commit -m "chore: add the project files"`.
- On no, stop. **Never push with zero commits.**

**5. Check what the push will publish.** Every commit on the branch goes
up, and a file deleted since is still in its history.
```bash
git -c core.quotePath=false log --format= --name-only HEAD | sort -u
```
- Pipe it through the same filter. A hit: stop, and name the file and the commit
  that added it (`git log --format='%h %s' --diff-filter=A -- <file>`).
  Getting it out means rewriting history, which is the user's call.
- Build output: not a stop. Say so in the report.
- Uncommitted changes and untracked files aren't pushed. Note them for the
  report.

**6. Check the branch.** `git branch --show-current` must print `main`.
- Empty is a detached HEAD: stop. There is no branch to push, so no "as it
  is" option; the user checks one out.
- Anything else (`master`, a feature branch): stop, and say which branch
  would be pushed. Renaming it is the user's call. If they say push it as it
  is, carry on, with that branch as `<branch>` in steps 8 to 10.

**7. Check `origin`.**
```bash
git remote get-url origin          # must fail: no origin yet
```
`origin` exists: stop, and show `git remote -v`.

**8. Create and push.**
```bash
gh repo create "$name" --private --source=. --remote=origin --push
```
The name is passed rather than left to default, so it is the name step 2
checked. If this fails, don't run it again: look at what it got done
(`gh repo view "<user>/$name"`, `git remote -v`), stop, and report that.
A repo that was made but not pushed is pushed with `git push -u origin <branch>`
once the cause is fixed.

**9. Verify.**
```bash
gh repo view "<user>/$name" --json visibility,url,defaultBranchRef
git remote -v
git rev-parse --abbrev-ref '@{upstream}'
```
- `visibility` is `PRIVATE`. If not, stop and say so plainly.
- `origin` points at the new repo, and the upstream is `origin/<branch>`.

**10. Report**, in this order:
- the URL, and that it's private
- `<branch>` tracks `origin/<branch>`, and how many commits went up
- whether this made the git repo or the first commit
- what wasn't pushed: uncommitted changes, untracked files, other local
  branches (`git branch --format='%(refname:short)'`)
- build output in the pushed history, if any

## What mustn't be pushed

Secrets. Paths go in on stdin, hits come out (the second `grep` drops
templates, names ending `.example`, `.sample` or `.template`):
```bash
grep -iE -e '(^|/)\.env(\.[^/]*)?$' \
         -e '(^|/)id_(rsa|dsa|ecdsa|ed25519)$' \
         -e '\.(pem|key|p12|pfx|jks|keystore|tfvars)$' \
         -e '\.tfstate(\.backup)?$' \
         -e '(^|/)(credentials|secrets?)(\.[^/]*)?$' \
         -e '(^|/)\.(npmrc|pypirc|netrc|envrc|git-credentials)$' \
  | grep -viE '\.(example|sample|template)$'
```
Look inside any other file that looks like it holds keys or tokens.

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
| The name is taken on the account | nothing changed | the user's call |
| `gh repo view` fails another way | nothing changed | rerun once it answers |
| The user says no to the first commit | `git init` done, nothing committed | commit, then rerun |
| A secret in what would be pushed | nothing new | the user cleans the history, or says it isn't a secret |
| The branch isn't `main` | nothing new | the user renames it, or says push it as it is |
| HEAD is detached | nothing new | the user checks out a branch, then reruns |
| `origin` already exists | nothing new | the user's call: it may already be on GitHub |
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
