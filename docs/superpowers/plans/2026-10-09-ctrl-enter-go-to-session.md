# Ctrl+Enter Goes to an Active Task's Session Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the keyboard task panel, Ctrl+Enter on an active task with a live Claude presses Go to session, as `g` does, and gives the keyboard back.

**Architecture:** `Action::advance` in `src/panel/actions.rs` picks the button Ctrl+Enter presses from a card's `TaskState`; it gains an arm that returns `Session` for an active task with `has_session`, ahead of the planned/unplanned arms. `Action::hint` already names whatever `advance` returns, so the hint reads `Ctrl+Enter: Go to session` with no change. `PanelState::advance` in `src/panel/state.rs` spawns that button's command while keeping the keyboard; for `Session` it instead goes through `on_press`, which is exactly what `g` does (give the keyboard back, then spawn `task session <uuid>`).

**Tech Stack:** Rust (`cargo test`), unit tests in the `#[cfg(test)] mod tests` at the bottom of each file.

**Spec:** Taskwarrior task `048a446e-0e81-469d-8929-2f0201e94c02` — read it with `task rc.json.array=on 048a446e-0e81-469d-8929-2f0201e94c02 export`. Its annotations:
- Goal: Ctrl+Enter on an active task's card goes to that task's Claude session in herdr, the same as pressing `g` (Go to session).
- Decided: An active task with a live Claude gets Go to session from Ctrl+Enter, whether or not it is planned. This replaces Refine on active tasks that have no plan.
- Decided: An active task with no live Claude keeps today's behaviour: Refine if it has no plan, nothing if it does.
- Decided: When Ctrl+Enter presses Go to session, the panel hands the keyboard back, as `g` does. Refine and Start working still keep the keyboard.
- Done when: unit tests in `actions.rs` and `state.rs` cover: Ctrl+Enter on an active card with a live Claude spawns `task session <uuid>` and releases the keyboard; the hint reads `Ctrl+Enter: Go to session`; an active card with no session behaves as it does today. `cargo test` passes.
- Out of scope: changing what plain Enter or `g` does, and starting a Claude for an active task that has none.

**Note on the spec's Steps line:** it says "Make `PanelState::advance` release the keyboard for an action that doesn't keep it". Refine and Start working also have `keeps_keyboard: false` in `src/actions.rs:144-145`, so that rule would release the keyboard for them too, against the Decided line that they keep it. This plan follows the Decided line: only Session releases, by going through `on_press` as `g` does. Do not change any `keeps_keyboard` value in `src/actions.rs`.

## Global Constraints

- Ctrl+Enter on an active task with a live Claude → Go to session, planned or not. On an active task without one → Refine if unplanned, nothing if planned (unchanged). Waiting, finished, and not-yet-started tasks: unchanged (a not-started task with a refine Claude open still gets Refine or Start working).
- Ctrl+Enter's Go to session gives the keyboard back, producing the same effects as `g`; Refine and Start working keep the keyboard and focus, with their notification, as today.
- Do not change plain Enter, `g`, `Action::row`, or anything in `src/actions.rs`.
- Commit messages: Conventional Commits, `feat(panel): …`, subject ≤ 72 chars, imperative, lowercase, no full stop; end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

---

### Task 1: `Action::advance` picks Go to session on an active task with a Claude

**Files:**
- Modify: `src/panel/actions.rs:33-45` (`advance` and its doc comment)
- Test: `src/panel/actions.rs` tests module — new tests after `ctrl_enter_refines_an_unplanned_task_and_starts_a_planned_one` (~line 160) and after `the_body_of_a_task_ctrl_enter_leaves_alone_hints_only_space` (~line 375)

**Interfaces:**
- Consumes: `TaskState { active, waiting, finished, planned, up_next, has_session }` from `crate::actions`; test helpers `on_list`, `active`, `waiting`, `finished`, `with_claude`, `hint` already in the tests module.
- Produces: `Action::advance(state: TaskState) -> Option<Action>` now returns `Some(Session)` when `state.active && state.has_session` (and not waiting/finished). Task 2 relies on this.

- [ ] **Step 1: Write the failing tests**

In `src/panel/actions.rs`'s tests module, after `ctrl_enter_refines_an_unplanned_task_and_starts_a_planned_one`, add:

```rust
    /// An active task with a live Claude goes to it, planned or not; one
    /// with none is left as it was: Refine without a plan, nothing with one.
    #[test]
    fn ctrl_enter_goes_to_the_session_of_an_active_task_with_a_claude() {
        assert_eq!(Action::advance(with_claude(active(false))), Some(Session));
        assert_eq!(Action::advance(with_claude(active(true))), Some(Session));
        assert_eq!(Action::advance(active(false)), Some(Refine));
        assert_eq!(Action::advance(active(true)), None);
        // A refine open on a task not yet started is not a task being worked.
        assert_eq!(Action::advance(with_claude(on_list(false))), Some(Refine));
        assert_eq!(Action::advance(with_claude(on_list(true))), Some(Start));
        assert_eq!(Action::advance(with_claude(waiting(true))), None);
        assert_eq!(Action::advance(with_claude(finished(true))), None);
    }
```

After `the_body_of_a_task_ctrl_enter_leaves_alone_hints_only_space`, add:

```rust
    /// The hint names Go to session in Ctrl+Enter's place, on the body and
    /// on a button.
    #[test]
    fn an_active_task_with_a_claude_hints_ctrl_enter_go_to_session() {
        for planned in [false, true] {
            let state = with_claude(active(planned));
            assert_eq!(hint(state, None), "Space: view notes · Ctrl+Enter: Go to session", "planned={planned}");
            assert_eq!(hint(state, Some(Session)), "g: Go to session · Ctrl+Enter: Go to session", "planned={planned}");
            assert_eq!(hint(state, Some(Stop)), "t: Stop · Ctrl+Enter: Go to session", "planned={planned}");
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test ctrl_enter_goes_to_the_session_of_an_active_task` then `cargo test an_active_task_with_a_claude_hints`

Expected: both FAIL — the first with `left: Some(Refine)` / `right: Some(Session)`, the second with the hint missing or reading `Ctrl+Enter: Refine`.

- [ ] **Step 3: Implement**

Replace `advance` and its doc comment in `src/panel/actions.rs` (currently lines 33-45) with:

```rust
    /// The button Ctrl+Enter presses for a card: Refine until the task has a
    /// plan, then Start working; once it is being worked, Go to session while
    /// a Claude is on it, planned or not. Nothing on a planned task being
    /// worked with no Claude, nor on a waiting or finished task, which have
    /// no step to take.
    pub fn advance(state: TaskState) -> Option<Action> {
        match state {
            TaskState { finished: true, .. } => None,
            TaskState { waiting: true, .. } => None,
            TaskState { active: true, has_session: true, .. } => Some(Session),
            TaskState { active: true, planned: true, .. } => None,
            TaskState { planned: true, .. } => Some(Start),
            _ => Some(Refine),
        }
    }
```

The arm order matters: the `has_session` arm must come before the `planned: true` arms.

- [ ] **Step 4: Run the module's tests to verify they pass**

Run: `cargo test panel::actions`
Expected: PASS, including the existing `ctrl_enter_only_presses_a_button_the_card_has` (Session is in `row` whenever `has_session` and on the list) and `every_hint_fits_beside_its_row` (the longest new hint, `g: Go to session · Ctrl+Enter: Go to session`, is 44 characters; an 11-button row has room for 53).

- [ ] **Step 5: Commit**

```bash
git add src/panel/actions.rs
git commit -m "$(cat <<'EOF'
feat(panel): pick go to session for ctrl+enter on a worked task

An active task with a live Claude now gets Go to session from
Ctrl+Enter, planned or not, in place of Refine or nothing. One with no
Claude keeps Refine without a plan and nothing with one.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Ctrl+Enter's Go to session gives the keyboard back, and the docs say so

**Files:**
- Modify: `src/panel/state.rs:878-891` (`PanelState::advance` and its doc comment)
- Modify: `src/panel/keys.rs:36-40` (doc comment on `KeyAction::Advance`)
- Modify: `README.md:21` and `README.md:88-92` (Ctrl+Enter text)
- Modify: `CONTEXT.md:105-110` (Action row: what Ctrl+Enter does)
- Test: `src/panel/state.rs` tests module — new tests after `ctrl_enter_does_nothing_on_a_planned_task_being_worked` (~line 1501)

**Interfaces:**
- Consumes: `Action::advance` returning `Some(Session)` for an active task with a live Claude (Task 1). In `state.rs`: `PanelState::on_press(&mut self, uuid: &str, slot: Slot) -> Vec<Effect>` (gives the keyboard back for a button that doesn't keep it, then `Effect::Spawn(action.args(uuid))`), `PanelState::take_keyboard(&mut self, agents: Vec<String>) -> bool`, `PanelState::card_hint(&self, uuid: &str) -> String`, `PanelState::keyboard() -> bool`, `crate::names::work_agent(uuid: &str) -> String` (the live agent name that makes `has_session` true). Test helpers `card`, `keyboard`, `key`, `focused` exist in the tests module.
- Produces: `fn advance(&mut self) -> Vec<Effect>` (was `&self`).

- [ ] **Step 1: Write the failing tests**

In `src/panel/state.rs`'s tests module, after `ctrl_enter_does_nothing_on_a_planned_task_being_worked`, add:

```rust
    /// An active card with a live Claude on it, planned or not, with the
    /// keyboard and the focus on its body.
    fn worked_with_claude(planned: bool) -> PanelState {
        let mut active = card("a", Status::Active);
        active.planned = planned;
        let mut state = PanelState::default();
        state.set_cards(&[active]);
        assert!(state.take_keyboard(vec![crate::names::work_agent("a")]));
        state
    }

    /// Ctrl+Enter on a worked task with a Claude does what g does: the
    /// keyboard back, then `task session <uuid>`.
    #[test]
    fn ctrl_enter_goes_to_the_session_giving_the_keyboard_back() {
        for planned in [false, true] {
            let mut state = worked_with_claude(planned);
            assert_eq!(state.card_hint("a"), "Space: view notes · Ctrl+Enter: Go to session", "planned={planned}");
            let effects = key(&mut state, KeyAction::Advance);
            assert_eq!(
                effects,
                vec![Effect::Render, Effect::Release, Effect::Spawn(vec!["task".into(), "session".into(), "a".into()])],
                "planned={planned}"
            );
            assert!(!state.keyboard(), "planned={planned}");
            let mut by_letter = worked_with_claude(planned);
            assert_eq!(key(&mut by_letter, KeyAction::Run(Action::Session)), effects, "the same as g");
        }
    }

    /// With no Claude on it, an unplanned worked task still refines and
    /// keeps the keyboard, as before.
    #[test]
    fn ctrl_enter_still_refines_a_worked_task_with_no_claude() {
        let mut state = keyboard(vec![card("a", Status::Active)]);
        assert_eq!(state.card_hint("a"), "Space: view notes · Ctrl+Enter: Refine");
        assert_eq!(
            key(&mut state, KeyAction::Advance),
            vec![Effect::Notify("Refine: a".into()), Effect::Spawn(Action::Refine.args("a"))],
        );
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }
```

(`Effect::Spawn` holds a `Vec<String>`, and `Action::Session.args("a")` returns `vec!["task", "session", "a"]` as `String`s — `src/actions.rs:249-251`. The literal vec pins the spec's `task session <uuid>`.)

- [ ] **Step 2: Run the tests to verify the first fails**

Run: `cargo test ctrl_enter_goes_to_the_session_giving_the_keyboard_back`
Expected: FAIL — the effects are `[Notify("Go to session: a"), Spawn([...])]` with no `Render, Release`.

Run: `cargo test ctrl_enter_still_refines_a_worked_task_with_no_claude`
Expected: PASS already (it pins today's behaviour).

- [ ] **Step 3: Implement**

Replace `PanelState::advance` and its doc comment in `src/panel/state.rs` (currently lines 878-891) with:

```rust
    /// Ctrl+Enter: the focused card's Refine, or its Start once it is
    /// planned, keeping the keyboard and the focus so the user can go on down
    /// the list — not through `on_press`, which would give the keyboard
    /// back. Go to session, on a worked task with a Claude, leaves the list
    /// for the session, so it goes through `on_press` and gives the keyboard
    /// back, as g does. Like the letters, only a button the card has.
    fn advance(&mut self) -> Vec<Effect> {
        let shown = self.visible();
        let Some(card) = self.current(&shown).map(|at| &shown[at]) else { return Vec::new() };
        let Some(uuid) = &card.card.uuid else { return Vec::new() };
        let Some(action) = card.state.and_then(Action::advance) else { return Vec::new() };
        if !card.actions.contains(&action) {
            return Vec::new();
        }
        if action == Action::Session {
            return self.on_press(uuid, Slot::Button(action));
        }
        vec![Effect::Notify(format!("{}: {}", action.label(false), card.card.text)), Effect::Spawn(action.args(uuid))]
    }
```

`shown` is an owned `Vec` from `visible()`, so `uuid` borrows it, not `self`, and `self.on_press` can take `&mut self`. If the borrow checker still objects, use `let uuid = uuid.clone();` before the `if` and pass `&uuid`.

The call site `KeyAction::Advance => self.advance(),` (~line 518) is already in a `&mut self` method; it needs no change.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test panel::state`
Expected: PASS, including the existing `ctrl_enter_refines_or_starts_keeping_the_keyboard_and_the_focus` and `ctrl_enter_does_nothing_on_a_planned_task_being_worked`.

- [ ] **Step 5: Update the doc comment on `KeyAction::Advance`**

In `src/panel/keys.rs`, replace:

```rust
    /// Ctrl+Enter: move the focused card's task on a step — refine it, or
    /// start working on it once it is planned — keeping the keyboard, so the
    /// list stays up to pick the next one.
    Advance,
```

with:

```rust
    /// Ctrl+Enter: move the focused card's task on a step — refine it, or
    /// start working on it once it is planned — keeping the keyboard, so the
    /// list stays up to pick the next one. On a task being worked with a
    /// Claude on it, go to that session instead, giving the keyboard back as
    /// g does.
    Advance,
```

- [ ] **Step 6: Update the README**

In `README.md` line 21 (the `Mod+Alt+Ctrl+T` table row), replace:

```
Ctrl+Enter refines the focused task, or starts working on it once it is planned, and keeps the list up.
```

with:

```
Ctrl+Enter refines the focused task, or starts working on it once it is planned, and keeps the list up; on a task already being worked with a Claude on it, it goes to that session.
```

In `README.md` lines 88-92, replace:

```
Ctrl+Enter and Ctrl+Delete act on the focused card, whichever of its buttons is
focused, without giving the keyboard back. Ctrl+Enter refines the task, or
starts working on it once it is planned, and the panel stays up with the same
card focused, so you can carry on down the list. It does nothing on a task that
is already being worked, or on a waiting or finished task. Ctrl+Delete deletes
```

with:

```
Ctrl+Enter and Ctrl+Delete act on the focused card, whichever of its buttons is
focused. Ctrl+Enter refines the task, or starts working on it once it is
planned, and the panel keeps the keyboard with the same card focused, so you
can carry on down the list. On a task already being worked with a live Claude,
planned or not, it goes to that session, as `g` does, and gives the keyboard
back. On a worked task with no Claude it refines the task if it has no plan
and does nothing if it has one; it does nothing on a waiting or finished task.
Ctrl+Delete deletes
```

Then read the following lines (93-96) and rewrap the paragraph to ~80 columns so the Ctrl+Delete sentence flows on; Ctrl+Delete's own words stay as they are (it still keeps the keyboard — "with the list kept up" already says so).

- [ ] **Step 7: Update CONTEXT.md**

In `CONTEXT.md`'s Action row entry, replace:

```
notes` or `Space: hide notes`; and what Ctrl+Enter does to the task. The keys
every card shares, Enter or Space and Ctrl+Delete, are on a line under the list
instead.
```

with:

```
notes` or `Space: hide notes`; and what Ctrl+Enter does to the task: Refine
until it has a plan, then Start working, and Go to session once it is being
worked with a live Claude. The keys every card shares, Enter or Space and
Ctrl+Delete, are on a line under the list instead.
```

- [ ] **Step 8: Run the whole suite**

Run: `cargo test`
Expected: PASS, no failures. Also `cargo build` with no new warnings.

- [ ] **Step 9: Commit**

```bash
git add src/panel/state.rs src/panel/keys.rs README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
feat(panel): give the keyboard back when ctrl+enter goes to a session

Ctrl+Enter's Go to session leaves the list for the task's Claude, so it
now does what g does: hands the keyboard back, then opens the session.
Refine and Start working still keep the keyboard. The README, CONTEXT
and the Advance key's doc say what Ctrl+Enter does on a worked task.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
