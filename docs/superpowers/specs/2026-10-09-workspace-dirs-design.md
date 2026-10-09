# Workspace and Dirs: one answer to "which workspace, which folder", and project.rs split

Date: 2026-10-09. Taskwarrior task `2d0c1baf`. Closes v3/v4 review cards D2 and D6.

## Problem

- "Which workspace" is answered three ways (the focused one, this terminal's session, whichever is asking) by nine functions at the root of `lib.rs`, beside two niri wrappers in `niri.rs`. Each session-opening entry point (`refine::launch`, `work::launch`, `work::set_up_here`, `link::go_to`, `Session::for_workspace`) takes the workspace as a bare `&str` and derives its session name and folder itself.
- `HOME` is read in eight files; `~/Projects` is joined in three; the folder rules `start_dir` and `project_from_cwd` live in `session.rs`, which is about herdr.
- `project.rs` (751 lines; 427 of code, 32 tests) holds five concerns: typed-name resolution, the Projects list with open and move, fzf/substring matching, `PATH` lookup, and the ghostty/herdr/VS Code argv. `Session::open` calls `project::project_terminal_command` while `project.rs` calls `session::herdr_session_name`, a cycle the D1 review flagged.

## Decisions

1. **`src/dirs.rs`, a `Dirs` value, is the one place that reads `HOME`.** `Dirs::from_env()` refuses without `HOME` ("HOME is unset") and reads `XDG_DATA_HOME` and `XDG_CACHE_HOME`, dropping a relative one as the XDG spec says. It answers `home()`, `projects()` (`~/Projects`), `data()` (XDG data or `~/.local/share`), `cache()` (XDG cache or `~/.cache`), `start_dir(workspace)` and `project_from_cwd(cwd)` (both moved from `session.rs`, rules unchanged). `Dirs::at(home)` builds one without XDG overrides, for tests; `with_data(p)`/`with_cache(p)` set them. "One lookup" means one definition: each entry point that needs paths builds one `Dirs` where it needs it.
2. **`src/workspace.rs`, a `Workspace` value, is always a named niri workspace with a usable tag.** `Workspace::named(name)` applies today's two refusals in today's order and words ("This workspace has no name — Mod+Alt+W opens a project on a named one."; "Workspace name '{name}' has no usable tag characters."). `focused()`, `of_session()` and `of_caller()` carry today's three policies, with today's error texts byte for byte. `name()`, `tag()`, `session_name()` (`session::herdr_session_name`) and `folder(&Dirs)` (`dirs.start_dir`) are its readers. `workspace::name_default()` replaces `lib::workspace_default`. The session-or-folder choice is split into two pure functions, `from_session(session, names)` and `from_folder(dirs, cwd, names)`, so it is unit-tested for the first time. `lib.rs` keeps only its module list and doc.
3. **Callers take `&Workspace`.** `main.rs` decides the policy once per command (`Tag`: `of_session`/`focused`; `Add`/`Move`: `of_caller().tag()`; `Start`/`Refine`/`Session`: `of_caller()`; `--here --workspace <name>`: `named(name)`) and passes the value to `refine::launch`, `work::launch`, `work::set_up_here`, `link::go_to`, and they to `Session::for_workspace(&Workspace)`, `link::live_agent(_names)`. The daemon uses `Workspace::focused()` for the panel's refusal text and `Workspace::named` for the agent-name lookup. `main::terminal` keeps opening on an unnamed workspace: it uses `Dirs::start_dir` on the focused name or `""`, not a `Workspace`.
4. **`project.rs` splits three ways.** `src/programs.rs` takes the external programs: `on_path`/`found_in`, `EDITOR`, `SESSION_MANAGER`, `terminal_command`, `project_terminal_command`, and `startup_commands(dir, session: &str)`, which now takes the herdr session name so the module depends on nothing internal. `src/panel/matching.rs` takes `MATCHER`, `fzf_matches`, `substring_matches`, beside the panel's injected `Matcher` type. `project.rs` keeps `Resolved`/`normalize`/`resolve`, `move_destinations`, `Projects`/`Row`/`Choice`/`load`/`rows`/`move_destinations`/`choice`/`open`/`start`, `list`/`folders_in`, `destination`/`move_task`, `open_args`. `start` computes the session name with `session::herdr_session_name(&name)` (not `Workspace::named`, which would refuse a tagless folder name today's open accepts). Dependencies after: `session → programs`, `project → session, programs, dirs`, `programs → nothing internal`. The cycle is gone.
5. **Nothing the user sees changes.** Every error and notification string is byte-identical; the herdr/niri argv is unchanged; the `--here --workspace` shape is unchanged; the three integration suites pass without edits.

## Out of scope

D5 (agent-name prefixes), any new workspace policy, `tag --session` semantics, the one-Dirs-per-process threading (each entry point builds its own), moving `tag::workspace_tag`.

## Tests

- `dirs.rs`: XDG precedence and the relative-path rule for `data()` and `cache()`; `start_dir` chain and `project_from_cwd` (moved from `session.rs`, adapted to `Dirs::at`).
- `workspace.rs`: `named` refusals (empty, tagless, both texts); `from_session` match/mismatch text; `from_folder` under `~/Projects/<name>` matching a workspace, under `~/Projects/<other>` with no workspace (text), outside `~/Projects` (text).
- Moved with their code: `programs` (`found_in`, argv), `matching` (substring; fzf test if one exists), `ideas::file`, `refine_mod_dir`, `github::cache_path` adapted to `Dirs`.
- Integration: `tests/typed_task_number.rs`, `tests/link_pane.rs`, `tests/write_path.rs` unchanged and green.

## Docs

CONTEXT.md **Workspace tag** entry: one sentence that `workspace::Workspace` is the named workspace with a usable tag, built by the focused, session or caller policy. **Session** entry unchanged. No README/llms.txt change (no user-visible change).
