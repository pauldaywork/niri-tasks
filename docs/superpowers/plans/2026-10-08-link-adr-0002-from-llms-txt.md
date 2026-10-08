# Link ADR 0002 from llms.txt Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An agent reading `llms.txt` can find ADR 0002, the record of why Refine keeps its sandbox fence and a mod takes the write.

**Architecture:** One bullet added to `llms.txt`'s `## Optional` section, between the existing ADR 0001 and ADR 0003 bullets and in their shape. No code changes; the existing `llms_txt_*` tests in `src/main.rs` guard the file's shape and command coverage.

**Tech Stack:** Markdown (llms.txt), Rust tests via `cargo test`.

**Spec:** Taskwarrior task `0a344d09-ff1a-42ff-b582-e1141311d5ec` ("docs: Link ADR 0002 from llms.txt"); read it with `task rc.json.array=on 0a344d09 export`. Its notes are the spec.

## Global Constraints

- Bullet shape, copied from the neighbours: relative link whose text is the path, a colon, a short "why ..." line.
- Suggested wording from the spec: "why Refine's write and approval moved into a mod tool while its sandbox fence stays".
- Commit subject, verbatim from the spec: `docs(llms): link adr 0002`. End the commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Out of scope: linking ADRs from the README (it mentions only 0001) and rewording the `refine-task` line under `## Skills`.
- Work in this worktree branch, `task/docs-link-adr-0002-from-llms-txt-0a344d09`; `finish-worktree` lands it on `main`.

---

### Task 1: Add the ADR 0002 bullet

**Files:**
- Modify: `llms.txt` (the `## Optional` section near the end; match on the quoted text, not a line number)
- Test: `src/main.rs` (existing `llms_txt_runs_every_subcommand_and_flag` and `llms_txt_has_the_llmstxt_shape`; no new test)

**Interfaces:**
- Consumes: the file `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, which already exists.
- Produces: nothing later tasks rely on.

- [ ] **Step 1: Confirm the gap**

Run: `grep -n 'docs/adr/' llms.txt`
Expected: two lines, 0001 then 0003, no 0002.

- [ ] **Step 2: Add the bullet**

In `llms.txt`, replace:

```markdown
- [docs/adr/0001-task-panel-in-gtk-not-quickshell.md](docs/adr/0001-task-panel-in-gtk-not-quickshell.md): why the task panel is drawn in GTK
- [docs/adr/0003-the-daemon-draws-and-the-cli-asks.md](docs/adr/0003-the-daemon-draws-and-the-cli-asks.md): why only the daemon draws windows, and why fuzzel left
```

with:

```markdown
- [docs/adr/0001-task-panel-in-gtk-not-quickshell.md](docs/adr/0001-task-panel-in-gtk-not-quickshell.md): why the task panel is drawn in GTK
- [docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md](docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md): why Refine's write and approval moved into a mod tool while its sandbox fence stays
- [docs/adr/0003-the-daemon-draws-and-the-cli-asks.md](docs/adr/0003-the-daemon-draws-and-the-cli-asks.md): why only the daemon draws windows, and why fuzzel left
```

- [ ] **Step 3: Check all three ADRs are linked, in number order, and the link resolves**

Run: `grep -o 'docs/adr/[0-9]\{4\}[^)]*\.md' llms.txt | sort -u | xargs ls`
Expected: the three ADR paths listed, 0001, 0002, 0003, and no "No such file" error.

Run: `grep -n 'docs/adr/' llms.txt`
Expected: three lines with ascending line numbers, 0001 then 0002 then 0003.

- [ ] **Step 4: Run the tests**

Run: `cargo test llms_txt`
Expected: `llms_txt_runs_every_subcommand_and_flag` and `llms_txt_has_the_llmstxt_shape` pass.

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add llms.txt
git commit -F - <<'EOF'
docs(llms): link adr 0002

llms.txt's Optional section linked ADRs 0001 and 0003 but skipped 0002,
so an agent reading it could not find why Refine keeps its sandbox
fence while a mod takes the write.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```
