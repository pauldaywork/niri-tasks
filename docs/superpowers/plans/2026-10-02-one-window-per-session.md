# One Window per herdr Session — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine, Start working and Go to session focus the ghostty window already attached to the workspace's herdr session, and open another only when there is none.

**Architecture:** `refine::show_session_window` finds the session's window by title. Today it looks only for a title ending in `": <first herdr workspace label>"`. herdr's title shows the *focused* herdr workspace, which after Start working is the task's worktree (`task/<slug>-<uuid8>`), so the lookup misses and attaches a duplicate client. The fix is to match a title ending in `": <label>"` for **any** of the session's herdr workspace labels. `herdr::first_workspace_label` becomes `herdr::workspace_labels`, and `find_session_window` takes a slice of labels.

**Tech Stack:** Rust (anyhow, serde_json, niri_ipc), herdr 0.9.1, ghostty 1.3.1, niri.

**Spec:** Taskwarrior task `1f80be1c-2266-466b-88bf-3b088a4f8739`. Read it with `task rc.json.array=on 1f80be1c-2266-466b-88bf-3b088a4f8739 export`. Its annotations (Goal, Context ×3, Decided, Steps, Done when, Out of scope) are the spec.

## Global Constraints

- Find the window by something that doesn't change with the focused herdr workspace. Matching **any of the session's workspace labels** is the chosen way; the research below says why the process tree isn't used.
- A new window opens only when no ghostty window's title ends in `": <label>"` for one of the session's labels.
- Unit tests cover a title showing a worktree workspace.
- Out of scope: closing duplicate windows that are already open, and changing the user's herdr `window_title`.
- Every public item gets a doc comment that says *why*. Match the surrounding code's comment density and naming.
- Commit messages: a plain sentence in the imperative, no `feat:` prefix, ending with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Research: why labels and not the process tree (verified 2026-10-02 on this machine)

- **ghostty runs single-instance.** `pgrep -a ghostty` shows one process, `/usr/bin/ghostty --gtk-single-instance=true --initial-window=false`, and `niri msg -j windows` gives every ghostty window the same `pid` (11572). `ghostty +new-window` (`project::terminal_command`) asks that process for another window. Every `herdr --session <s>` client is a child of that one pid, so the process tree can show that a session has *a* client but not *which window* it is in. ghostty 1.3.1 on Linux has no IPC to list its windows or their ptys.
- **herdr does not list its clients.** herdr 0.9.1's `api snapshot` and `api schema` hold no per-client data (no pid, tty or title), only the `client_window_title` event herdr uses to *set* the title.
- **The title holds a workspace label.** herdr's `--default-config` documents `window_title = "{hostname}: {workspace}"`. Its tokens are `{hostname}`, `{workspace}`, `{tab}`, `{pane}` and `{terminal_title}`; none names the session. Live titles look like `paul-msi-ubuntu: niri-tasks` and `paul-msi-ubuntu: task/document-every-niritasks-command-add-6a970973`.
- **Focus is per client.** Two niri-tasks windows are open now: one titled with the project label and one with a worktree label. `workspace list` reports one `focused` workspace, so no single label covers every client. Matching any label covers them all.
- **`workspace list` shape** (herdr 0.9.1): `{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"workspace_id":"w1","label":"niri-tasks","focused":false},{"workspace_id":"wF","label":"task/document-every-niritasks-command-add-6a970973","focused":true}]}}`.
- **Known limit.** A label that two sessions share, such as one a user typed into both, could match the other session's window. Project labels are unique per project, and worktree labels end in the task's uuid8, so the labels niri-tasks creates don't collide. A user who changes `window_title` still gets an extra window rather than a focus, as now.

## File Structure

- **Modify `src/herdr.rs`**: replace `first_workspace_label` (lines 255–263) with `workspace_labels`, and update its test (lines 443–461).
- **Modify `src/refine.rs`**: `find_session_window` (lines 227–244) takes `&[String]` labels; `show_session_window` (lines 246–268) passes every label; add tests to the `find_session_window` tests (lines 493–519).

`first_workspace_label` has no other callers (`grep -rn first_workspace_label src tests`), so it is replaced rather than kept.

---

### Task 1: Find the session's window by any of its workspace labels

**Files:**
- Modify: `src/herdr.rs:255-263`, `src/herdr.rs:443-461`
- Modify: `src/refine.rs:227-268`, `src/refine.rs:493-519`

**Interfaces:**
- Consumes: `niri::windows() -> Result<Vec<niri_ipc::Window>>`, `niri::focus_window(u64)`, `niri::spawn(Vec<String>)`, `project::project_terminal_command(dir, s, true)`.
- Produces:
  - `herdr::workspace_labels(list: &serde_json::Value) -> Vec<String>`: every workspace's label in list order; empty when there are none.
  - `refine::find_session_window(windows: &[WindowInfo], labels: &[String]) -> Option<u64>` (private).

- [ ] **Step 1: Write the failing herdr test**

In `src/herdr.rs`, replace the test `workspace_label_and_tab_id_are_read_from_herdr_responses` with:

```rust
    /// Shapes copied from herdr 0.9.1's real responses, like `ids_are_read_…` above.
    #[test]
    fn workspace_labels_and_tab_id_are_read_from_herdr_responses() {
        let list: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"workspace_id":"w1","label":"niri-tasks","focused":false},{"workspace_id":"wF","label":"task/fix-it-6a970973","focused":true}]}}"#,
        ).unwrap();
        assert_eq!(workspace_labels(&list), vec!["niri-tasks", "task/fix-it-6a970973"]);

        let empty: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[]}}"#,
        ).unwrap();
        assert!(workspace_labels(&empty).is_empty());

        let created: Value = serde_json::from_str(
            r#"{"id":"cli:tab:create","result":{"root_pane":{"pane_id":"w1:p2","tab_id":"w1:t2"},"tab":{"tab_id":"w1:t2"},"type":"tab_created"}}"#,
        ).unwrap();
        assert_eq!(created_tab_id(&created).as_deref(), Some("w1:t2"));
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --lib herdr::tests::workspace_labels_and_tab_id 2>&1 | tail -15`
Expected: compile error, ``cannot find function `workspace_labels` ``.

The lib won't compile again until Step 6, because `src/refine.rs` still calls `first_workspace_label`. The refine tests are written first, in Step 4, so Steps 3–6 make up one red–green cycle.

- [ ] **Step 3: Replace `first_workspace_label` with `workspace_labels`**

In `src/herdr.rs`, replace the whole `first_workspace_label` item (doc comment included) with:

```rust
/// Every herdr workspace's label in the session, in herdr's order.
///
/// herdr's default `window_title` is `"{hostname}: {workspace}"`, where
/// `{workspace}` is the label of the herdr workspace that client has focused:
/// the project's own workspace, or a task's worktree after Start working.
/// `refine` looks for the outer terminal by any of these labels, because
/// which one the title shows depends on what the user last focused.
pub fn workspace_labels(list: &Value) -> Vec<String> {
    list["result"]["workspaces"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|w| w["label"].as_str().map(str::to_string))
        .collect()
}
```

- [ ] **Step 4: Write the failing refine tests**

In `src/refine.rs`'s `mod tests`, replace the four `find_session_window` tests (from `const GHOSTTY` to the end of `a_label_that_is_a_suffix_of_another_label_does_not_match`) with the following. The old tests pass `&str`; these pass label slices, and two new ones cover a worktree title.

```rust
    const GHOSTTY: &str = "com.mitchellh.ghostty";

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn finds_the_ghostty_window_whose_title_ends_with_the_label() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), Some(1));
    }

    /// After Start working, herdr's title names the task's worktree
    /// workspace, not the project's: the window is still the session's.
    #[test]
    fn finds_the_window_showing_a_worktree_workspace() {
        let windows = [(4, Some(GHOSTTY), Some("paul-msi-ubuntu: task/fix-it-6a970973"))];
        let session = labels(&["niri-tasks", "task/fix-it-6a970973"]);
        assert_eq!(find_session_window(&windows, &session), Some(4));
    }

    /// Another session's worktree is not this session's window, even though
    /// both titles start with `task/`.
    #[test]
    fn a_worktree_of_another_session_does_not_match() {
        let windows = [(4, Some(GHOSTTY), Some("paul-msi-ubuntu: task/other-1234abcd"))];
        let session = labels(&["niri-tasks", "task/fix-it-6a970973"]);
        assert_eq!(find_session_window(&windows, &session), None);
    }

    #[test]
    fn no_window_matches_a_different_label() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: other"))];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), None);
    }

    #[test]
    fn a_session_with_no_workspaces_matches_no_window() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, &[]), None);
    }

    #[test]
    fn a_different_app_id_with_the_matching_title_does_not_match() {
        let windows = [(1, Some("org.wezfurlong.wezterm"), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), None);
    }

    /// "tasks" must not match a title ending "niri-tasks" — a label that is a
    /// suffix of another workspace's label is not the same workspace.
    #[test]
    fn a_label_that_is_a_suffix_of_another_label_does_not_match() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: niri-tasks"))];
        assert_eq!(find_session_window(&windows, &labels(&["tasks"])), None);
    }
```

- [ ] **Step 5: Change `find_session_window` to take every label**

In `src/refine.rs`, replace `find_session_window` and its doc comment (from `/// The ghostty window already showing` to the end of the function) with:

```rust
/// The ghostty window already showing session `labels` belongs to, if niri
/// has one open.
///
/// herdr sets the outer terminal's title to its `window_title`, default
/// `"{hostname}: {workspace}"`, where `{workspace}` is the label of whichever
/// herdr workspace that client has focused: the project's, or a task's
/// worktree after Start working. So a session's ghostty window's title ends
/// with `": <label>"` for one of `labels`, the session's workspace labels.
/// The process tree can't say which window a client is in, because ghostty
/// runs every window from one process. This depends on herdr's default title:
/// a user who changes `window_title` just costs themselves an extra attached
/// terminal window rather than a focus, which is harmless.
fn find_session_window(windows: &[WindowInfo], labels: &[String]) -> Option<u64> {
    let suffixes: Vec<String> = labels.iter().map(|l| format!(": {l}")).collect();
    windows
        .iter()
        .find(|(_, app_id, title)| {
            *app_id == Some("com.mitchellh.ghostty")
                && title.is_some_and(|t| suffixes.iter().any(|s| t.ends_with(s)))
        })
        .map(|(id, _, _)| *id)
}
```

- [ ] **Step 6: Pass every label from `show_session_window`**

In `src/refine.rs`'s `show_session_window`, replace

```rust
    let Some(label) = herdr::first_workspace_label(list) else {
        return Ok(());
    };
```

with

```rust
    let labels = herdr::workspace_labels(list);
    if labels.is_empty() {
        return Ok(());
    }
```

and replace `match find_session_window(&info, &label) {` with `match find_session_window(&info, &labels) {`. Leave its doc comment as it is; "no title to look for" still holds for a session with no workspace.

- [ ] **Step 7: Run the tests to see them pass**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked|error" `
Expected: every `test result:` line says `ok`; no `FAILED`, `panicked` or `error` lines.

Run: `cargo clippy --all-targets 2>&1 | tail -3`
Expected: no warnings in `src/herdr.rs` or `src/refine.rs`.

- [ ] **Step 8: Commit**

```bash
git add src/herdr.rs src/refine.rs
git commit -m "$(cat <<'EOF'
Find a session's window by any of its herdr workspace labels, so a title showing a worktree still matches

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Check by hand that no duplicate opens

This needs the user's real niri desktop and herdr, which the nested-niri e2e scripts don't start. The executor asks the user to run it, or runs it with the user's permission on a workspace they choose. No code changes. If a duplicate still opens, use superpowers:systematic-debugging rather than guessing.

**Files:** none.

- [ ] **Step 1: Install the built binary**

Run: `cargo install --path . --locked 2>&1 | tail -2`, or `./install.sh` if the user prefers that route. Check how `install.sh` installs before running it.
Expected: `niritasks` on `PATH` is this branch's build (`which niritasks`).

- [ ] **Step 2: Count the windows before**

On a project workspace whose herdr session already has one window, run:
`niri msg -j windows | jq '[.[] | select(.app_id=="com.mitchellh.ghostty")] | length'`
Note the count.

- [ ] **Step 3: Reproduce the original scenario**

On that workspace: Start working on a scratch task, so herdr focuses its `task/…` worktree and the window title ends in `: task/…`. Then Refine another task on the same workspace, then Go to session on the first task. After each one, re-run the count from Step 2.
Expected: the count never goes up; each action focuses the existing window. Before this change, the Refine added one.

- [ ] **Step 4: Check that a closed window still comes back**

Close that session's ghostty window, leaving the herdr server running, then Refine a task on that workspace.
Expected: one new ghostty window opens, attached to the session (count back to its Step 2 value).

- [ ] **Step 5: Tidy up**

Remove the scratch task and its worktree the way the user prefers. Don't close duplicate windows that were already open before this check; that is out of scope.
