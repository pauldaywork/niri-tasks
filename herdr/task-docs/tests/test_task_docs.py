import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import task_docs  # noqa: E402

# herdr: logs each call as a JSON list, answers `agent get <target>` from the
# FAKE_AGENTS map, exiting 1 for a target it does not hold, as herdr does, and
# `pane list` with the FAKE_PANES list.
FAKE_HERDR = """#!/usr/bin/env python3
import json, os, sys
with open(os.environ["FAKE_HERDR_LOG"], "a") as log:
    log.write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[1:3] == ["agent", "get"]:
    agent = json.load(open(os.environ["FAKE_AGENTS"])).get(sys.argv[3])
    if agent is None:
        print('{"error":{"code":"agent_not_found"}}', file=sys.stderr)
        sys.exit(1)
    print(json.dumps({"result": {"agent": agent}}))
if sys.argv[1:3] == ["pane", "list"]:
    print(json.dumps({"result": {"panes": json.load(open(os.environ["FAKE_PANES"]))}}))
"""

# A command that writes its arguments, one per line, to the file named by $LOG_VAR.
FAKE_LOGGER = """#!/bin/sh
printf '%s\\n' "$@" >> "$LOG_VAR"
"""

# niritasks: logs its arguments, prints two documents.
FAKE_NIRITASKS = """#!/bin/sh
printf '%s\\n' "$@" > "$FAKE_NIRITASKS_LOG"
printf 'Report · 2026-10-11\\t/r/a.md\\n'
printf 'Plan · b.md\\t/p/b.md\\n'
"""


# niritasks: logs its arguments; exits $FAKE_NIRITASKS_EXIT with an error, as
# niritasks does with no daemon.
FAKE_NIRITASKS_PANEL = """#!/bin/sh
printf '%s\\n' "$@" > "$FAKE_NIRITASKS_LOG"
if [ -n "$FAKE_NIRITASKS_EXIT" ]; then
    echo "Error: The niri-tasks daemon is not running" >&2
    exit "$FAKE_NIRITASKS_EXIT"
fi
"""

# niritasks: appends its arguments as one line per call; `task peek <uuid8>`
# exits $FAKE_UUID_EXIT, as niritasks does for an unknown task, and every call
# exits $FAKE_PEEK_EXIT, as with no daemon.
FAKE_NIRITASKS_PEEK = """#!/bin/sh
echo "$*" >> "$FAKE_NIRITASKS_LOG"
if [ -n "$FAKE_PEEK_EXIT" ]; then
    echo "Error: The niri-tasks daemon is not running" >&2
    exit "$FAKE_PEEK_EXIT"
fi
if [ -n "$FAKE_UUID_EXIT" ] && [ "$3" != "--clear" ]; then
    echo "Error: task not found" >&2
    exit "$FAKE_UUID_EXIT"
fi
"""


def fake(folder, name, body):
    path = Path(folder) / name
    path.write_text(body)
    path.chmod(0o755)
    return path


class PluginTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.herdr_log = self.tmp / "herdr.log"
        self.notify_log = self.tmp / "notify.log"
        self.open_log = self.tmp / "xdg-open.log"
        self.agents = self.tmp / "agents.json"
        self.with_agents({})
        self.panes = self.tmp / "panes.json"
        self.with_panes([])
        fake(self.bin, "herdr", FAKE_HERDR)
        fake(self.bin, "notify-send", FAKE_LOGGER.replace("LOG_VAR", "FAKE_NOTIFY_LOG"))
        fake(self.bin, "xdg-open", FAKE_LOGGER.replace("LOG_VAR", "FAKE_OPEN_LOG"))
        self.folder = self.tmp / "work"
        self.folder.mkdir()
        env = {
            "PATH": f"{self.bin}:/usr/bin:/bin",
            "HERDR_BIN_PATH": str(self.bin / "herdr"),
            "FAKE_HERDR_LOG": str(self.herdr_log),
            "FAKE_AGENTS": str(self.agents),
            "FAKE_PANES": str(self.panes),
            "HERDR_SESSION": "web",
            "FAKE_NOTIFY_LOG": str(self.notify_log),
            "FAKE_OPEN_LOG": str(self.open_log),
            "XDG_CONFIG_HOME": str(self.tmp / "config"),
            "HERDR_PLUGIN_ID": "task-docs",
            "HERDR_PLUGIN_STATE_DIR": str(self.tmp / "state"),
            "HERDR_PANE_ID": "w1:p1",
            "HERDR_PLUGIN_CONTEXT_JSON": json.dumps(
                {"focused_pane_id": "w1:p1", "focused_pane_cwd": str(self.folder), "focused_pane_agent": "claude"}
            ),
        }
        patcher = mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)

    def with_agents(self, agents):
        self.agents.write_text(json.dumps(agents))

    def with_panes(self, panes):
        self.panes.write_text(json.dumps(panes))

    def with_reviewer(self):
        fake(self.bin, "plannotator-tui", "#!/bin/sh\n")

    def on_branch(self, branch):
        subprocess.run(["git", "init", "-q", "-b", branch, str(self.folder)], check=True)

    def calls(self):
        if not self.herdr_log.exists():
            return []
        return [json.loads(line) for line in self.herdr_log.read_text().splitlines()]

    def pane_opens(self):
        return [c for c in self.calls() if c[:3] == ["plugin", "pane", "open"]]

    def notified(self):
        return self.notify_log.read_text() if self.notify_log.exists() else ""


class FindingTheTask(PluginTest):
    def test_an_agent_named_for_a_task_gives_its_uuid8(self):
        self.assertEqual(task_docs.uuid8_of_agent("work-1234abcd"), "1234abcd")
        self.assertEqual(task_docs.uuid8_of_agent("task-1234abcd"), "1234abcd")
        self.assertEqual(task_docs.uuid8_of_agent("report-1234abcd"), "1234abcd")
        self.assertIsNone(task_docs.uuid8_of_agent("reviewer"))
        self.assertIsNone(task_docs.uuid8_of_agent(None))

    def test_a_task_branch_gives_its_uuid8(self):
        self.on_branch("task/fix-the-peek-1234abcd")
        self.assertEqual(task_docs.uuid8_of_branch(self.folder), "1234abcd")

    def test_another_branch_or_no_repo_gives_none(self):
        self.assertIsNone(task_docs.uuid8_of_branch(self.folder))
        self.on_branch("main")
        self.assertIsNone(task_docs.uuid8_of_branch(self.folder))

    def test_the_panes_agent_name_comes_before_its_branch(self):
        self.on_branch("task/other-99999999")
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        self.assertEqual(task_docs.find_task("w1:p1", self.folder), "1234abcd")

    def test_the_tasks_claude_is_this_pane_then_work_then_task(self):
        self.with_agents({"w1:p1": {"name": "task-1234abcd", "pane_id": "w1:p1"},
                          "work-1234abcd": {"name": "work-1234abcd", "pane_id": "w2:p1"}})
        self.assertEqual(task_docs.task_claude("w1:p1", "1234abcd"), "w1:p1")
        self.assertEqual(task_docs.task_claude("w1:p9", "1234abcd"), "w2:p1")
        self.with_agents({"task-1234abcd": {"name": "task-1234abcd", "pane_id": "w3:p1"}})
        self.assertEqual(task_docs.task_claude(None, "1234abcd"), "w3:p1")
        self.with_agents({})
        self.assertIsNone(task_docs.task_claude("w1:p1", "1234abcd"))


class TheAction(PluginTest):
    def test_a_pane_with_no_task_says_so_and_opens_nothing(self):
        self.with_reviewer()
        task_docs.open_action()
        self.assertIn("Task docs: no task here", self.notified())
        self.assertEqual(self.pane_opens(), [])

    def test_without_plannotator_tui_it_says_herdr_annotate_full_is_needed(self):
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        task_docs.open_action()
        self.assertIn("plannotator/herdr-annotate", self.notified())
        self.assertEqual(self.pane_opens(), [])

    def test_a_task_with_no_claude_says_so(self):
        self.with_reviewer()
        self.on_branch("task/fix-1234abcd")
        task_docs.open_action()
        self.assertIn("no Claude on this task", self.notified())
        self.assertEqual(self.pane_opens(), [])

    def test_a_tasks_pane_opens_the_list_over_it(self):
        self.with_reviewer()
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        task_docs.open_action()
        self.assertEqual(self.pane_opens(), [[
            "plugin", "pane", "open", "--plugin", "task-docs", "--entrypoint", "pick",
            "--placement", "overlay", "--cwd", str(self.folder), "--focus",
            "--env", "TASK_DOCS_TASK=1234abcd", "--env", "TASK_DOCS_CLAUDE=w1:p1",
        ]])

    def test_a_pane_in_a_subfolder_opens_the_list_at_the_repo_root(self):
        self.with_reviewer()
        self.on_branch("task/fix-1234abcd")
        sub = self.folder / "src" / "deep"
        sub.mkdir(parents=True)
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        with mock.patch.dict(os.environ, {"HERDR_PLUGIN_CONTEXT_JSON": json.dumps(
                {"focused_pane_id": "w1:p1", "focused_pane_cwd": str(sub)})}):
            task_docs.open_action()
        argv = self.pane_opens()[0]
        self.assertEqual(argv[argv.index("--cwd") + 1], str(self.folder.resolve()))

    def test_without_a_pane_folder_the_plugins_own_folder_is_not_guessed(self):
        self.with_reviewer()
        with mock.patch.dict(os.environ, {"HERDR_PLUGIN_CONTEXT_JSON": json.dumps({"focused_pane_id": "w1:p1"})}):
            old = os.getcwd()
            os.chdir(ROOT)  # the plugin's folder is inside a task branch checkout
            self.addCleanup(os.chdir, old)
            task_docs.open_action()
        self.assertIn("Task docs: no task here", self.notified())
        self.assertEqual(self.pane_opens(), [])

    def test_from_a_shell_on_the_tasks_branch_the_comments_go_to_its_working_claude(self):
        self.with_reviewer()
        self.on_branch("task/fix-1234abcd")
        self.with_agents({"work-1234abcd": {"name": "work-1234abcd", "pane_id": "w5:p2"}})
        task_docs.open_action()
        self.assertIn("TASK_DOCS_CLAUDE=w5:p2", self.pane_opens()[0])

    def test_plannotator_tui_from_herdr_annotate_full_counts(self):
        staged = self.tmp / "config/herdr/plugins/github/annotate-abc123/bin/plannotator-tui.exe"
        staged.parent.mkdir(parents=True)
        fake(staged.parent, staged.name, "#!/bin/sh\n")
        self.assertEqual(task_docs.reviewer(), str(staged))


class ThePanelAction(PluginTest):
    def setUp(self):
        super().setUp()
        fake(self.bin, "niritasks", FAKE_NIRITASKS_PANEL)
        self.niritasks_log = self.tmp / "niritasks.log"
        patcher = mock.patch.dict(os.environ, {"FAKE_NIRITASKS_LOG": str(self.niritasks_log)})
        patcher.start()
        self.addCleanup(patcher.stop)

    def asked(self):
        return self.niritasks_log.read_text().split("\n")[:-1] if self.niritasks_log.exists() else None

    def test_a_tasks_pane_opens_the_panel_on_its_task(self):
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        task_docs.panel_action()
        self.assertEqual(self.asked(), ["task", "panel", "1234abcd"])
        self.assertEqual(self.notified(), "")

    def test_a_shell_on_the_tasks_branch_opens_it_too(self):
        self.on_branch("task/fix-1234abcd")
        task_docs.panel_action()
        self.assertEqual(self.asked(), ["task", "panel", "1234abcd"])

    def test_it_needs_no_claude_or_plannotator(self):
        self.on_branch("task/fix-1234abcd")
        task_docs.panel_action()
        self.assertNotIn("plannotator", self.notified())
        self.assertNotIn("no Claude", self.notified())
        self.assertEqual(self.asked(), ["task", "panel", "1234abcd"])

    def test_a_pane_with_no_task_says_so_and_opens_nothing(self):
        task_docs.panel_action()
        self.assertIn("Task panel: no task here", self.notified())
        self.assertIsNone(self.asked())

    def test_a_failing_niritasks_says_why(self):
        self.on_branch("task/fix-1234abcd")
        with mock.patch.dict(os.environ, {"FAKE_NIRITASKS_EXIT": "1"}):
            task_docs.panel_action()
        self.assertIn("Task panel: could not open the panel", self.notified())
        self.assertIn("daemon is not running", self.notified())

    def test_without_niritasks_it_says_so(self):
        self.on_branch("task/fix-1234abcd")
        (self.bin / "niritasks").unlink()
        task_docs.panel_action()
        self.assertIn("Task panel: niritasks not found", self.notified())


class TheFocusHook(PluginTest):
    def setUp(self):
        super().setUp()
        fake(self.bin, "niritasks", FAKE_NIRITASKS_PEEK)
        self.niritasks_log = self.tmp / "niritasks.log"
        patcher = mock.patch.dict(os.environ, {"FAKE_NIRITASKS_LOG": str(self.niritasks_log)})
        patcher.start()
        self.addCleanup(patcher.stop)

    def asked(self):
        return self.niritasks_log.read_text().splitlines() if self.niritasks_log.exists() else []

    def focus(self, pane, cwd=None):
        """herdr's panes with `pane` the focused one, in `cwd`, the work folder by default."""
        self.with_panes([
            {"pane_id": "w1:p9", "focused": False, "cwd": "/elsewhere", "foreground_cwd": "/elsewhere"},
            {"pane_id": pane, "focused": True, "cwd": "/launched", "foreground_cwd": str(cwd or self.folder)},
        ])

    def test_a_focused_tasks_pane_peeks_at_its_task(self):
        self.with_agents({"w1:p2": {"name": "work-1234abcd", "pane_id": "w1:p2"}})
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd"])
        self.assertEqual(self.notified(), "")

    def test_a_focused_shell_on_the_tasks_branch_peeks_at_it(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd"])

    def test_a_pane_on_no_task_clears_the_peek(self):
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek --clear"])

    def test_the_same_pane_again_is_ignored(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd"])

    def test_another_pane_peeks_again(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        self.focus("w1:p3")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek 1234abcd"])

    def test_with_no_pane_focused_it_does_nothing(self):
        self.with_panes([{"pane_id": "w1:p2", "focused": False, "cwd": str(self.folder)}])
        task_docs.focused_action()
        self.assertEqual(self.asked(), [])

    def test_a_task_niritasks_does_not_know_clears_the_peek(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        with mock.patch.dict(os.environ, {"FAKE_UUID_EXIT": "1"}):
            task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek --clear"])

    def test_a_failing_niritasks_notifies_nothing_and_tries_again_next_time(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        with mock.patch.dict(os.environ, {"FAKE_PEEK_EXIT": "1"}):
            task_docs.focused_action()
            task_docs.focused_action()
        self.assertEqual(self.notified(), "")
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek --clear"] * 2)

    def test_without_niritasks_it_notifies_nothing(self):
        (self.bin / "niritasks").unlink()
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.notified(), "")

    def test_each_session_remembers_its_own_pane(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        with mock.patch.dict(os.environ, {"HERDR_SESSION": "api"}):
            task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek 1234abcd"])


class TheManifest(unittest.TestCase):
    def test_selecting_a_pane_a_tab_or_a_workspace_runs_focused(self):
        import tomllib
        manifest = tomllib.loads((ROOT / "herdr-plugin.toml").read_text())
        hooks = {event["on"]: event["command"] for event in manifest["events"]}
        focused = ["python3", "task_docs.py", "focused"]
        self.assertEqual(hooks, {"pane.focused": focused, "tab.focused": focused, "workspace.focused": focused})


class TheCommandLine(unittest.TestCase):
    def test_panel_is_a_command(self):
        with mock.patch.object(task_docs, "panel_action") as panel:
            self.assertEqual(task_docs.main(["task_docs.py", "panel"]), 0)
        panel.assert_called_once_with()

    def test_focused_is_a_command(self):
        with mock.patch.object(task_docs, "focused_action") as focused:
            self.assertEqual(task_docs.main(["task_docs.py", "focused"]), 0)
        focused.assert_called_once_with()


class Notifying(PluginTest):
    def test_a_notify_send_that_fails_falls_back_to_herdr(self):
        fake(self.bin, "notify-send", "#!/bin/sh\nexit 1\n")
        task_docs.notify("hello")
        self.assertIn(["notification", "show", "Tasks", "--body", "hello", "--sound", "none"], self.calls())

    def test_a_notify_send_that_works_is_enough(self):
        task_docs.notify("hello")
        self.assertEqual(self.notified(), "Tasks\nhello\n")
        self.assertEqual(self.calls(), [])


class ThePicker(PluginTest):
    def test_lines_split_at_the_first_tab_and_others_are_skipped(self):
        self.assertEqual(
            task_docs.parse_docs("Report · 2026-10-11\t/r/a\tb.md\nno tab here\n"),
            [("Report · 2026-10-11", "/r/a\tb.md")],
        )

    def test_it_asks_niritasks_for_the_tasks_documents(self):
        fake(self.bin, "niritasks", FAKE_NIRITASKS)
        log = self.tmp / "niritasks.log"
        with mock.patch.dict(os.environ, {"FAKE_NIRITASKS_LOG": str(log)}):
            items = task_docs.docs("1234abcd", self.folder)
        self.assertEqual(items, [("Report · 2026-10-11", "/r/a.md"), ("Plan · b.md", "/p/b.md")])
        self.assertEqual(log.read_text().split("\n")[:-1], [
            "task", "docs", "1234abcd", "--project", str(self.folder), "--notes-dir", str(self.tmp / "state"),
        ])

    def test_the_pick_opens_beside_the_claude_named_by_the_action(self):
        fake(self.bin, "niritasks", FAKE_NIRITASKS)
        env = {"TASK_DOCS_TASK": "1234abcd", "TASK_DOCS_CLAUDE": "w1:p1", "FAKE_NIRITASKS_LOG": str(self.tmp / "n.log")}
        with mock.patch.dict(os.environ, env), \
                mock.patch.object(task_docs, "choose", return_value="/r/a.md") as choose, \
                mock.patch.object(task_docs, "open_doc") as open_doc:
            task_docs.pick()
        choose.assert_called_once_with([("Report · 2026-10-11", "/r/a.md"), ("Plan · b.md", "/p/b.md")])
        open_doc.assert_called_once_with("/r/a.md", "w1:p1")

    def test_esc_opens_nothing(self):
        fake(self.bin, "niritasks", FAKE_NIRITASKS)
        env = {"TASK_DOCS_TASK": "1234abcd", "TASK_DOCS_CLAUDE": "w1:p1", "FAKE_NIRITASKS_LOG": str(self.tmp / "n.log")}
        with mock.patch.dict(os.environ, env), \
                mock.patch.object(task_docs, "choose", return_value=None), \
                mock.patch.object(task_docs, "open_doc") as open_doc:
            task_docs.pick()
        open_doc.assert_not_called()


class OpeningADocument(PluginTest):
    def test_markdown_opens_in_plannotator_tui_right_of_the_claude_and_delivers_to_it(self):
        task_docs.open_doc("/r/refine-a.md", "w1:p1")
        self.assertEqual(self.pane_opens(), [[
            "plugin", "pane", "open", "--plugin", "task-docs", "--entrypoint", "review",
            "--placement", "split", "--direction", "right", "--target-pane", "w1:p1", "--focus",
            "--cwd", "/r", "--env", "PLANNOTATOR_TUI_FILE=/r/refine-a.md",
            "--env", "PLANNOTATOR_TUI_DELIVER_TO=w1:p1",
        ]])

    def test_a_report_with_no_twin_opens_in_the_browser(self):
        task_docs.open_doc("/r/refine-a.html", "w1:p1")
        for _ in range(50):
            if self.open_log.exists():
                break
            time.sleep(0.05)
        self.assertEqual(self.open_log.read_text(), "/r/refine-a.html\n")
        self.assertEqual(self.pane_opens(), [])


if __name__ == "__main__":
    unittest.main()
