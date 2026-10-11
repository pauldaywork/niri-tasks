# Unattended Reads Never Denied Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An unattended refine and report (`niritasks task refine <uuid> --unattended`, the task box's Add & all) never has a read-only step denied: the two skills keep Bash to shapes that run unasked, and the session's Read tool may read wherever sandboxed Bash already can.

**Architecture:** Two small changes, no new code paths. The skills gain one sentence each telling an unattended run what shell shapes Claude Code's permission parser refuses under `--permission-prompts none` (a `for` loop's `$f`, a quoted `"$f"`, a `;` chain ending in a glob were the three real denials; every single plain command, pipeline, `~` path and glob ran in three probes on 2.1.296), and to open files with the Read tool. `refine::session_settings` replaces its two skill-folder Read allows (`Read(~/.claude/skills/**)` plus one `Read(//<real folder>/**)` per `skill_dirs` entry) with one `Read(//**)`, so the Read tool reads anywhere, under the existing credential deny rules, which win over allows. `skill_dirs`, its parameter, its callers and its tests go. Nothing in the fence, the mod or the unattended runner changes.

**Tech Stack:** Rust (`cargo test`), the two Markdown skills under `.claude/skills/`, the TypeScript refine mod in `claude/refine-mod` (`claude plugin validate`, `claude plugin test`, unchanged but re-run), Claude Code 2.1.296 (`claude -p --permission-prompts none`), Taskwarrior 3.5.

**Spec:** Taskwarrior task `82ff74c6-f7f3-49d5-95db-b597a16cb6fa`, "fix: Stop an unattended run's reads being denied". Read it with `task rc.json.array=on 82ff74c6-f7f3-49d5-95db-b597a16cb6fa export`; its notes are the spec and are quoted in **Global Constraints**.

## Global Constraints

- **The skill fix is prose, not fence:** one sentence each in `refine-task`'s `auto` bullet and `report-task`'s unattended note. Not plan mode, not auto mode, not a PermissionRequest hook (the hook is the fallback only if a run still shows a denial after this).
- **What the sentence says (from the spec, sharpened by the probes):** anything that would prompt is denied, so keep every shell command to one plain read-only pipeline of `ls`, `cat`, `grep`, `head`, `tail`, `find`, `wc` or `task`: no `$( )`, loops, variables (bare or quoted), `;` chains or glob paths; open files, in or out of the project, with the Read tool, which may read anywhere. The probes showed a quoted word (`echo "hello world"`, `grep -n "^==" file`) and a `~` path run fine alone, so the spec's "quoted words or ~-globs" is written as "variables (bare or quoted)" and "glob paths", which is what was actually refused.
- **The Read rule:** `Read(//**)` in `session_settings`, attended and unattended, in place of `Read(~/.claude/skills/**)` and every `skill_dirs` rule. `//` is Claude Code's absolute form; one `/` would be the project's. The `CREDENTIALS` deny rules stay and win: a probe with `Read(//**)` allowed read the run log, the installed catalogue through its link and the project's README, and was refused `~/.ssh/known_hosts` with "File is in a directory that is denied by your permission settings".
- **No new reach:** sandboxed Bash and WebFetch could already read and fetch the same files; the hidden sockets and credentials stay hidden from Bash by `hidden_paths`.
- **A refine session's tools are Read and Bash:** no Grep, no Glob; a skill must not point at them.
- **Docs:** one sentence in ADR 0002's `*2026-10-10:*` unattended addendum paragraph. README, CONTEXT.md and llms.txt mention neither the Read allows nor `skill_dirs`, so they do not change.
- **Done when:** an unattended refine and report on a throwaway task finishes with no denial in either session transcript under `~/.claude/projects/-home-paul-Projects-niri-tasks/` and the report step reads the run log; attended tabs unchanged apart from the Read tool no longer prompting outside the project; `cargo test` and `claude plugin test` green.
- **Out of scope:** switching the unattended run to auto or plan mode; a PermissionRequest hook; widening Bash allow rules; changing what the run log captures.
- **Skill changes go live only after landing:** the installed skills (`~/.claude/skills/*`) are links into `~/Projects/niri-tasks`, never a worktree, so a run started from this worktree uses the old skill text with the new Rust. Task 3 runs once from the worktree to prove the Read rule and once after `finish-worktree` to prove the sentences.
- **Transcript check:** grep for `no approval surface`, the phrase inside every automatic denial ("this session has no approval surface — nobody can answer a permission prompt here — so it was denied automatically"). Do not grep for `denied automatically`: a `task export` of the spec task echoes that phrase from its own notes into the transcript.
- **Commits:** Conventional Commits, subject at most 72 characters, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. The task's type is `fix`, scope `refine`; the ADR commit is `docs`.
- **Tests stay green at every commit.**

## File Structure

- `.claude/skills/refine-task/SKILL.md` (modify): the `auto` bullet in **3. Work it up**, lines 98–108, gains two sentences at its end.
- `.claude/skills/report-task/SKILL.md` (modify): the unattended paragraph in **4. Show it**, lines 87–92, gains two sentences at its end.
- `src/refine.rs` (modify): `session_settings` (lines 112–170: doc comment, signature, allow list), `skill_dirs` (lines 172–186, removed), the two callers (`launch` line 316, `unattended_steps` lines 556, 566, 571), the tests `the_session_may_search_the_web_but_not_read_credentials` (lines 760–780, rewritten), `each_resolved_skill_folder_is_allowed_beside_the_links` and `skill_dirs_resolve_the_installed_catalogue_and_skip_what_is_absent` (lines 782–814, removed), and every other `session_settings(...)` call in the tests module, which drops its `&[]` skill-dirs argument.
- `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` (modify): one sentence in the `*2026-10-10:*` unattended paragraph, lines 33–49.

---

### Task 1: The skills tell an unattended run what runs unasked

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md:98-108` (the `auto` bullet)
- Modify: `.claude/skills/report-task/SKILL.md:87-92` (the unattended paragraph)

**Interfaces:**
- Consumes: nothing from other tasks. The sentences name the Read tool and the eight commands only; no Grep or Glob.
- Produces: the two sentences Task 3's post-landing run checks for by their effect (zero denials). No code reads the skills.

- [ ] **Step 1: Add the sentences to refine-task's `auto` bullet**

In `.claude/skills/refine-task/SKILL.md`, the `auto` bullet ends:

```markdown
  report: the run builds one itself afterwards, with `report-task`. Grilling
  is never unattended.
```

Replace those two lines with:

```markdown
  report: the run builds one itself afterwards, with `report-task`. Grilling
  is never unattended. Nobody can answer a permission prompt either, so
  anything that would ask is denied: keep every shell command to one plain
  read-only pipeline of `ls`, `cat`, `grep`, `head`, `tail`, `find`, `wc`
  or `task` — no `$( )`, loops, variables (bare or quoted), `;` chains or
  glob paths — and open a file, in or out of the project, with the Read
  tool, which may read anywhere.
```

Run: `sed -n 98,116p .claude/skills/refine-task/SKILL.md`
Expected: the bullet reads as above, still indented two spaces under `- **`auto`:**`, with the next bullet-less paragraph `## 4. Propose` unchanged below it.

- [ ] **Step 2: Add the sentences to report-task's unattended paragraph**

In `.claude/skills/report-task/SKILL.md`, the paragraph in **4. Show it** ends:

```markdown
report is found later. Its answer then reads **Wrote the report to … and
linked it from the task** and does not mention opening the browser;
everything else is the same, and you still end at step 5.
```

Replace those three lines with:

```markdown
report is found later. Its answer then reads **Wrote the report to … and
linked it from the task** and does not mention opening the browser;
everything else is the same, and you still end at step 5. Nobody can answer
a permission prompt either, so anything that would ask is denied: keep
every shell command to one plain read-only pipeline of `ls`, `cat`, `grep`,
`head`, `tail`, `find`, `wc` or `task` — no `$( )`, loops, variables (bare
or quoted), `;` chains or glob paths — and open a file, in or out of the
project (the run's own log under `~/.local/share/niri-tasks/unattended/`,
a report under the reviews folder), with the Read tool, which may read
anywhere.
```

Run: `sed -n 87,101p .claude/skills/report-task/SKILL.md`
Expected: the paragraph reads as above and the `Its answer says what happened:` line follows it after one blank line.

- [ ] **Step 3: Check the skills still parse and name no missing tool**

Run: `head -12 .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md | grep -c "^name:"` then `grep -n "Grep\|Glob" .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md`
Expected: `2` (both frontmatters intact), then no output from the second command (the skills point at no tool a refine session lacks).

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md
git commit -m "fix(refine): keep an unattended run's shell reads to what runs unasked" -m "Under --permission-prompts none anything that would ask is denied, and the
first Add & all run had three read-only Bash calls refused by the
permission parser: a for loop's variable, a quoted variable and a ; chain
ending in a glob. Both skills now tell an unattended run to keep each
shell command to one plain read-only pipeline and to open files with the
Read tool instead.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: The Read tool may read anywhere sandboxed Bash can

**Files:**
- Modify: `src/refine.rs:112-186` (`session_settings` doc comment, signature and allow list; `skill_dirs` removed)
- Modify: `src/refine.rs:310-318` (`launch`'s call), `src/refine.rs:556-571` (`unattended_steps`'s calls)
- Test: `src/refine.rs` tests module (`the_session_may_search_the_web_but_not_read_credentials` rewritten; two tests removed; every `session_settings` call loses one argument)
- Modify: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md:33-49`

**Interfaces:**
- Consumes: `CREDENTIALS`, `REFINE_MOD`, `Mode`, `hidden_paths` (all exist and are unchanged).
- Produces: `pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path, mode: Mode, unattended: bool) -> String`, whose `permissions.allow` is exactly `["WebSearch", "WebFetch", "Bash(task *)", "Read(//**)"]`. `skill_dirs` no longer exists. Task 3 relies on the rule being in both the attended (`launch`) and unattended (`unattended_steps`) settings.

- [ ] **Step 1: Rewrite the allow-list test to expect `Read(//**)`**

In the `tests` module of `src/refine.rs`, replace the doc comment and body of `the_session_may_search_the_web_but_not_read_credentials` (the test starting `/// The web and the skill's reads run unasked, and so does the Read tool` around line 760) with:

```rust
    /// The web and the skill's reads run unasked, and so does the Read tool
    /// anywhere: the installed skills and the report catalogue, an
    /// unattended run's log and the reviews folder all lie outside the
    /// project, where the tool would otherwise ask and an unattended run has
    /// no one to answer, while sandboxed Bash can read them anyway. `//` is
    /// Claude Code's absolute form; one `/` would be the project's.
    /// Credentials still cannot be read through it, the one reader the Bash
    /// sandbox does not cover: a deny rule wins over an allow. No `python3`:
    /// the write is the mod's tool now.
    #[test]
    fn the_session_may_search_the_web_but_not_read_credentials() {
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], "u", Path::new("/r"), &[], Mode::Quick, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        for tool in ["WebSearch", "WebFetch", "Bash(task *)", "Read(//**)"] {
            assert!(allow.iter().any(|a| a == tool), "{tool} allowed");
        }
        assert_eq!(allow.len(), 4, "nothing else runs unasked");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("python3"))));
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("skills"))), "one rule for every folder, not a list");
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.iter().any(|d| d == "Read(~/.ssh/**)"));
        assert!(deny.iter().any(|d| d == "Read(~/.claude/.credentials.json)"));
        assert_eq!(deny.len(), CREDENTIALS.len());
    }
```

(The call still passes the `&[]` skill-dirs argument for now, so the file compiles; Step 3 drops it everywhere.)

- [ ] **Step 2: Run the test to see it fail**

Run: `cargo test --quiet the_session_may_search_the_web_but_not_read_credentials 2>&1 | tail -8`
Expected: FAIL with `Read(//**) allowed` (the assertion message), since the allow list still holds `Read(~/.claude/skills/**)`.

- [ ] **Step 3: Change `session_settings` and remove `skill_dirs`**

In `src/refine.rs`, replace the paragraph of `session_settings`'s doc comment that begins `/// The Read tool may read the installed skills too` and ends `/// CLAUDE_CONFIG_DIR elsewhere would bring the prompt back.` (lines 130–134) with:

```rust
/// The Read tool may read anywhere (`Read(//**)`: `//` is Claude Code's
/// absolute form, one `/` would be the project's), as sandboxed Bash
/// already can. Without it the tool asks outside the project, where the
/// installed skills, the report catalogue, an unattended run's log and the
/// reviews folder all live, and an unattended run has no one to answer.
/// The [`CREDENTIALS`] deny rules win over it, so nothing opens that Bash
/// could not read already. One rule for every project, not a list of
/// folders.
```

Replace the signature and the allow list (lines 142–147):

```rust
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path, skill_dirs: &[PathBuf], mode: Mode, unattended: bool) -> String {
    let deny: Vec<String> = CREDENTIALS.iter().map(|c| format!("Read({c})")).collect();
    let mut allow: Vec<String> =
        ["WebSearch", "WebFetch", "Bash(task *)", "Read(~/.claude/skills/**)"].iter().map(|s| s.to_string()).collect();
    // `//` is Claude Code's absolute form; one `/` would be the project's.
    allow.extend(skill_dirs.iter().map(|dir| format!("Read(//{}/**)", dir.display().to_string().trim_start_matches('/'))));
```

with:

```rust
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path, mode: Mode, unattended: bool) -> String {
    let deny: Vec<String> = CREDENTIALS.iter().map(|c| format!("Read({c})")).collect();
    let allow = ["WebSearch", "WebFetch", "Bash(task *)", "Read(//**)"];
```

Delete the whole `skill_dirs` function and its doc comment (from `/// The real folders of the installed skill files the session's Read tool` through the closing `}` of `pub fn skill_dirs`, lines 172–186).

In `launch`, delete the line `        &skill_dirs(home),` from the `session_settings(` call (line 316).

In `unattended_steps`, delete the line `    let skills = skill_dirs(home);` (line 556) and change both calls:

```rust
    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, &skills, Mode::Quick, true);
```
→
```rust
    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, Mode::Quick, true);
```
and
```rust
    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, &skills, Mode::Report, true);
```
→
```rust
    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, Mode::Report, true);
```

In the tests module, delete the two tests and their doc comments:
- `each_resolved_skill_folder_is_allowed_beside_the_links` (from `/// install.sh links the skills into the repo, and Claude Code lets the` to its closing `}`),
- `skill_dirs_resolve_the_installed_catalogue_and_skip_what_is_absent` (from `/// The catalogue's real folder, found through its installed link; none` to its closing `}`).

Then drop the `&[]` skill-dirs argument from every remaining `session_settings(` call in the tests module. All of them have the shape `Path::new("…"), &[], Mode::…` or `Path::new("…"), &[], mode, …`, so one substitution does it:

```bash
sed -i -E 's/(Path::new\("[^"]*"\)), &\[\], (Mode::|mode,)/\1, \2/g' src/refine.rs
```

Run: `grep -n "skill_dirs\|skills/\*\*\|&\[\], Mode\|&\[\], mode" src/refine.rs`
Expected: no output.

- [ ] **Step 4: Run the tests**

Run: `cargo test --quiet 2>&1 | tail -5`
Expected: all tests pass; `cargo build` warns about nothing (`home` is still used by `hidden_paths` in both callers). If the compiler reports an unused variable or import, fix that line before going on.

- [ ] **Step 5: Commit the Rust change**

```bash
git add src/refine.rs
git commit -m "fix(refine): let the Read tool read anywhere sandboxed Bash can" -m "The session's Read tool prompted outside the project, so an unattended
run could open neither its own log nor the reviews folder, and the two
skill-folder allows were a list that grew with every file linked into the
skills. One Read(//**) rule replaces them, under the credential deny
rules, which win over allows: no new reach, since sandboxed Bash and
WebFetch could already read the same files. skill_dirs and its tests go
with the rules it fed.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

- [ ] **Step 6: Add the ADR sentence**

In `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, the `*2026-10-10:*` unattended paragraph ends:

```markdown
credentials, the removed tools and the standing instruction, none of which
the mode touches. The task carries `+processing` while it runs, so the card
and the commands that start a Claude leave it alone meanwhile. The decision is unchanged.
```

Replace those three lines with:

```markdown
credentials, the removed tools and the standing instruction, none of which
the mode touches. The task carries `+processing` while it runs, so the card
and the commands that start a Claude leave it alone meanwhile. *2026-10-11:*
`session_settings` allows `Read(//**)`, attended and unattended, in place
of its skill-folder allows, so the Read tool reads wherever sandboxed Bash
already can (the run log, the reviews folder, the installed skills) under
the same credential deny rules, and the two skills tell an unattended run
to keep shell commands to plain read-only pipelines, since anything that
would prompt is denied. The decision is unchanged.
```

Run: `grep -c "Read(//\*\*)" docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`
Expected: `1`.

- [ ] **Step 7: Commit the ADR**

```bash
git add docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -m "docs(adr): note the Read(//**) allow and the unattended shell shapes" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: Prove it on a throwaway task

**Files:**
- None modified. Reads `~/.claude/projects/-home-paul-Projects-niri-tasks/*.jsonl`, `~/.local/share/niri-tasks/unattended/`, `~/.local/share/niri-tasks/reviews/`.

**Interfaces:**
- Consumes: Task 2's `session_settings` with `Read(//**)` (through `cargo run -- task refine <uuid> --unattended`); Task 1's sentences, live only after landing.
- Produces: the evidence the task's Done when asks for. Nothing later depends on it.

- [ ] **Step 1: Run the mod's checks and the Rust tests once more**

Run: `cargo test --quiet 2>&1 | tail -3 && cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -6; cd ../..`
Expected: `cargo test` reports `test result: ok`; `claude plugin validate` prints no error; `claude plugin test` ends with every test passing and none failing. The mod did not change, so a failure here is an environment problem (Claude Code version, `bun`), not this plan's: report it and stop.

- [ ] **Step 2: Add a throwaway task whose plan points at the run log**

```bash
task rc.confirmation=off add +niri_tasks -- 'chore: Throwaway check that an unattended run reads its log'
U=$(task rc.json.array=on +niri_tasks status:pending export | python3 -I -c 'import json,sys; print([t["uuid"] for t in json.load(sys.stdin) if t["description"].startswith("chore: Throwaway check")][0])')
task rc.confirmation=off "$U" annotate -- 'Goal: Confirm the newest run log under ~/.local/share/niri-tasks/unattended/ can be opened and its first line quoted; this task is deleted once checked.'
echo "$U"
```

Expected: `Created task N.`, then `Annotating task N …`, then the uuid. Keep `$U` for the following steps. (`--` ends Taskwarrior's attribute parsing, so the `chore:` type stays in the description; `rc.` overrides go before it.)

- [ ] **Step 3: Run the unattended refine and report from the worktree**

Run from the worktree root, with `claude` on `PATH` and nothing else on screen:

```bash
cargo run --quiet -- task refine "$U" --unattended; echo "exit $?"
```

Expected: a few minutes of silence, a desktop notification `Planned and reported: chore: Throwaway check …`, and `exit 0`. The two newest transcripts under `~/.claude/projects/-home-paul-Projects-niri-tasks/` are this run's refine and report steps. A non-zero exit names the step and the log under `~/.local/share/niri-tasks/unattended/`; read the log and the transcripts before anything else, and do not re-run blindly.

- [ ] **Step 4: Check the transcripts for denials and for the log being read**

```bash
P=~/.claude/projects/-home-paul-Projects-niri-tasks
ls -t "$P"/*.jsonl | head -2
for f in $(ls -t "$P"/*.jsonl | head -2); do echo "$f: $(grep -c 'no approval surface' "$f") denials, $(grep -c 'Path is outside allowed working directories' "$f") Read prompts, $(grep -o '"file_path":"/home/paul/.local/share/niri-tasks/unattended/[^"]*"' "$f" | sort -u | wc -l) Read calls on the run log"; done
```

Expected, for this worktree run (old skill text, new Read rule): both files show `0 Read prompts`, and the report step's file shows at least `1 Read calls on the run log`. The `denials` count should be `0` too; if it is not, print each denied command and its reason (the `What required approval:` sentence) with the script below and judge it: a shape the new sentences forbid (a loop, a `$var`, a chain with a glob) is expected until the skills land and is re-checked in Step 7; any other shape, or any Read tool denial, is a defect in this plan — stop and report it.

```bash
python3 -I - "$f" <<'EOF'
import json,sys
calls={}
for line in open(sys.argv[1]):
    try: o=json.loads(line)
    except ValueError: continue
    c=(o.get('message') or {}).get('content')
    if not isinstance(c,list): continue
    for b in c:
        if b.get('type')=='tool_use': calls[b['id']]=b.get('input',{})
        if b.get('type')=='tool_result':
            t=b.get('content')
            if isinstance(t,list): t=' '.join(x.get('text','') for x in t if isinstance(x,dict))
            if t and 'no approval surface' in t:
                inp=calls.get(b['tool_use_id'],{})
                print('CMD:',(inp.get('command') or inp.get('file_path') or str(inp))[:300])
                print('WHY:',t.split('What required approval:')[-1].strip()[:300],'\n')
EOF
```

- [ ] **Step 5: Check the task and clean up**

```bash
task rc.json.array=on "$U" export | python3 -I -c 'import json,sys; t=json.load(sys.stdin)[0]; print(sorted(t["tags"])); [print("-", a["description"][:120]) for a in t["annotations"]]'
```

Expected: tags `['niri_tasks', 'planned']` (no `processing`), notes `Goal:` … `Done when:` and a last `Report: /home/paul/.local/share/niri-tasks/reviews/refine-<uuid8>-<stamp>.html` whose file exists. Then remove the throwaway and what it wrote:

```bash
R=$(task rc.json.array=on "$U" export | python3 -I -c 'import json,sys; t=json.load(sys.stdin)[0]; print([a["description"][8:] for a in t["annotations"] if a["description"].startswith("Report: ")][-1])')
rm -f "$R"; ls -t ~/.local/share/niri-tasks/unattended/ | head -1
task rc.confirmation=off "$U" delete
```

Expected: `Deleted 1 task.` The run log named by `ls` may stay (the folder is for reading failures later) or go with `rm`; either is fine.

- [ ] **Step 6: Check an attended tab is unchanged**

Open a refine tab on any pending task from the panel (its card's **Refine**), and in it ask Claude to read `~/.local/share/niri-tasks/unattended/`'s newest log with the Read tool and then `~/.ssh/known_hosts`.
Expected: the first read opens with no permission prompt; the second is refused by the permission settings. Everything else about the tab — the sandbox, the write tool's question, the removed tools — is as before. Close the tab without writing the task.

- [ ] **Step 7: After landing, run once more with the new skill text**

This step runs after `finish-worktree` has merged the branch into `~/Projects/niri-tasks`, since the installed skills are links into that checkout. Then `install.sh` (which `cargo install`s the binary) and repeat Steps 2 to 5 with `niritasks task refine "$U" --unattended` in place of `cargo run --quiet -- …`.
Expected: `0 denials` and `0 Read prompts` in both transcripts, and at least one Read call on the run log in the report step's. That is the task's Done when; mark it done with `niritasks task status 82ff74c6-f7f3-49d5-95db-b597a16cb6fa completed`. If a denial remains, record its command and reason in a new `+niri_tasks` task (`fix: …`) naming the PermissionRequest hook as the fallback the spec chose, and do not widen any Bash allow rule.

---

## Self-review

- **Spec coverage.** Goal: the skills keep Bash to unasked shapes (Task 1), the Read tool reads anywhere Bash can (Task 2). Context: the three denial reasons are quoted in the sentences' prohibitions and in the Architecture note; the transcript, not the run log, is where a denial shows (Task 3 Step 4). Decided: fix the skills, not the fence; not plan, auto or a hook (Global Constraints, Task 3 Step 7's fallback). Decided: `Read(//**)` attended and unattended under the credential denies, subsuming `Read(~/.claude/skills/**)` and `skill_dirs`, which go with their tests (Task 2). Steps 1 to 3 of the spec map to Tasks 1, 2 and 3; the ADR line is Task 2 Step 6. Done when: Task 3 Steps 4 to 7. Out of scope is left out: no mode change, no hook, no Bash allow widening, no change to the run log.
- **Placeholder scan.** Every edit shows its before and after text; every command has its expected output; the only conditional ("if a denial remains") says exactly what to do.
- **Type consistency.** `session_settings(project, task_data, hidden, uuid, reports, mode, unattended)` is the signature in Task 2's Interfaces, Step 3's code and both callers; the tests' `sed` keeps `&[]` for `hidden` and drops only the one before `Mode::`/`mode,`. The allow list literal `["WebSearch", "WebFetch", "Bash(task *)", "Read(//**)"]` is the same in the test and the implementation.
- **A deviation, stated.** The spec's sentence lists "quoted words or ~-globs"; the plan writes "variables (bare or quoted)" and "glob paths", because the probes on 2.1.296 showed a quoted word and a `~` path run alone while a quoted `"$f"` and a chain ending in a glob were refused. The prohibitions still cover every real denial.
