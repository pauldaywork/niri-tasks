#!/usr/bin/env python3
"""herdr's task-docs plugin: annotate one of a task's documents in plannotator-tui,
or open the task panel on the task.

`task_docs.py open` is the action. It finds the task of the pane the key was
pressed in, from its agent's name (work-/task-/report-<uuid8>) or its folder's branch
(task/<slug>-<uuid8>), and that task's Claude, then opens the `pick` overlay.
`task_docs.py pick` runs in that overlay: fzf over `niritasks task docs`, and
the pick opens in the `review` pane, right of the task's Claude, where
plannotator-tui's Send makes the comments that Claude's next message. A report
made before reports had a Markdown twin opens in the browser instead.
`task_docs.py review` runs in that pane: plannotator-tui's own pane mode.
`task_docs.py panel` is the panel action: it finds the pane's task the same
way and runs `niritasks task panel <uuid8>`, which slides out the task panel
with the keyboard on that task's card.
`task_docs.py focused` is the focus hook herdr runs as a pane, a tab or a
workspace is selected: it finds the session's focused pane's task the same
way and runs `niritasks task peek <uuid8>`, or `--clear` for a pane on no
task, so that task's card alone peeks out of its workspace's task panel. Every
focus event peeks: the daemon ignores a peek it already shows. A failure goes
to stderr, herdr's plugin log, never to a notification.
"""

import glob
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

PLUGIN = os.environ.get("HERDR_PLUGIN_ID") or "task-docs"
AGENT = re.compile(r"^(?:work|task|report)-([0-9a-f]{8})$")
BRANCH = re.compile(r"^task/.+-([0-9a-f]{8})$")
FULL = "Install herdr-annotate Full, which ships it: herdr plugin install plannotator/herdr-annotate"
PREVIEW = (
    'case {2} in *.md) head -n 200 {2} ;; '
    '*) echo "Opens in the browser: a report made before reports had a Markdown twin." ;; esac'
)


def state_dir():
    """Where the task's notes are written as Markdown: herdr's state folder for this plugin."""
    return Path(os.environ.get("HERDR_PLUGIN_STATE_DIR") or Path.home() / ".local/state/herdr/plugins/task-docs")


def herdr(*args):
    """Run a herdr CLI command, capturing its output."""
    binary = os.environ.get("HERDR_BIN_PATH") or "herdr"
    return subprocess.run([binary, *args], capture_output=True, text=True, check=False)


def notify(body):
    """A desktop notification, as niritasks sends: herdr shows none for the tab in
    front, which is where this key is pressed. herdr's own when notify-send is
    missing or fails, so the message is not lost."""
    try:
        sent = subprocess.run(["notify-send", "Tasks", body], stdout=subprocess.DEVNULL,
                              stderr=subprocess.DEVNULL, check=False).returncode == 0
    except OSError:
        sent = False
    if not sent:
        herdr("notification", "show", "Tasks", "--body", body, "--sound", "none")


def context():
    """herdr's context for this invocation, or {} when it is missing or unreadable."""
    try:
        return json.loads(os.environ.get("HERDR_PLUGIN_CONTEXT_JSON") or "{}")
    except ValueError:
        return {}


def agent(target):
    """herdr's record of the agent in a pane, or of that name, or None."""
    result = herdr("agent", "get", target)
    if result.returncode != 0:
        return None
    try:
        return json.loads(result.stdout)["result"]["agent"]
    except (ValueError, KeyError, TypeError):
        return None


def uuid8_of_agent(name):
    """The task a work-/task-/report-<uuid8> agent is named for, or None."""
    match = AGENT.match(name or "")
    return match.group(1) if match else None


def uuid8_of_branch(cwd):
    """The task a task/<slug>-<uuid8> branch checked out in cwd is for, or None."""
    try:
        out = subprocess.run(["git", "-C", str(cwd), "branch", "--show-current"],
                             capture_output=True, text=True, check=False).stdout.strip()
    except OSError:
        return None
    match = BRANCH.match(out)
    return match.group(1) if match else None


def find_task(pane, cwd):
    """The uuid8 of the pane's task: its agent's name first, else its folder's branch."""
    here = agent(pane) if pane else None
    return uuid8_of_agent((here or {}).get("name")) or (uuid8_of_branch(cwd) if cwd else None)


def task_claude(pane, uuid8):
    """The pane of the task's Claude: this pane when its agent is named for the
    task, else the working Claude's (work-), else the refine's (task-)."""
    for target in (pane, f"work-{uuid8}", f"task-{uuid8}"):
        if not target:
            continue
        found = agent(target)
        if found and uuid8_of_agent(found.get("name")) == uuid8:
            return found.get("pane_id")
    return None


def reviewer():
    """plannotator-tui: on PATH, else the copy herdr-annotate Full stages in its plugin folder."""
    found = shutil.which("plannotator-tui")
    if found:
        return found
    config = Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config")
    for path in sorted(glob.glob(str(config / "herdr/plugins/github/annotate-*/bin/plannotator-tui.exe"))):
        if os.access(path, os.X_OK):
            return path
    return None


def toplevel(cwd):
    """The git repository root of cwd, or cwd itself when it is in none."""
    try:
        out = subprocess.run(["git", "-C", str(cwd), "rev-parse", "--show-toplevel"],
                             capture_output=True, text=True, check=False)
    except OSError:
        return cwd
    return out.stdout.strip() if out.returncode == 0 and out.stdout.strip() else cwd


def no_task(title):
    """What an action says in a pane with no task, under its own title."""
    notify(f"{title}: no task here. This pane runs no task's Claude (work-/task-/report-<uuid8>) "
           "and is not on a task/<slug>-<uuid8> branch.")


def open_action():
    """The task-docs.open action: open the task's documents over its pane, or say why not."""
    ctx = context()
    pane = os.environ.get("HERDR_PANE_ID") or ctx.get("focused_pane_id")
    cwd = ctx.get("focused_pane_cwd")
    uuid8 = find_task(pane, cwd)
    if not uuid8:
        no_task("Task docs")
        return
    if not reviewer():
        notify(f"Task docs: plannotator-tui is needed. {FULL}")
        return
    claude = task_claude(pane, uuid8)
    if not claude:
        notify(f"Task docs: no Claude on this task. Neither work-{uuid8} nor task-{uuid8} is running, "
               "so there is no one to send comments to.")
        return
    result = herdr("plugin", "pane", "open", "--plugin", PLUGIN, "--entrypoint", "pick",
                   "--placement", "overlay", *(["--cwd", str(toplevel(cwd))] if cwd else []), "--focus",
                   "--env", f"TASK_DOCS_TASK={uuid8}", "--env", f"TASK_DOCS_CLAUDE={claude}")
    if result.returncode != 0:
        notify(f"Task docs: could not open the list. {result.stderr.strip()[-300:]}")


def panel_action():
    """The task-docs.panel action: slide out the task panel with the keyboard,
    focused on the pane's task's card, or say why not. The daemon says when
    the card is not on the panel."""
    ctx = context()
    pane = os.environ.get("HERDR_PANE_ID") or ctx.get("focused_pane_id")
    cwd = ctx.get("focused_pane_cwd")
    uuid8 = find_task(pane, cwd)
    if not uuid8:
        no_task("Task panel")
        return
    try:
        result = subprocess.run(["niritasks", "task", "panel", uuid8],
                                capture_output=True, text=True, check=False)
    except OSError:
        notify("Task panel: niritasks not found on PATH.")
        return
    if result.returncode != 0:
        notify(f"Task panel: could not open the panel. {result.stderr.strip()[-300:]}")


def focused_pane():
    """The session's focused pane, as (pane id, its live folder), or None when
    herdr cannot say or no pane is focused. The focus events carry only ids, a
    tab's or a workspace's, so every one is resolved here the same way."""
    try:
        result = herdr("pane", "list")
    except OSError:
        print("task-docs focused: herdr not found", file=sys.stderr)
        return None
    if result.returncode != 0:
        return None
    try:
        panes = json.loads(result.stdout)["result"]["panes"]
    except (ValueError, KeyError, TypeError):
        return None
    for pane in panes:
        if pane.get("focused") and pane.get("pane_id"):
            return pane["pane_id"], pane.get("foreground_cwd") or pane.get("cwd")
    return None


def peek(*args):
    """`niritasks task peek` with these arguments; whether it worked. A failure
    goes to stderr, which herdr keeps in the plugin's log: a notification on
    every pane switch would be noise."""
    try:
        result = subprocess.run(["niritasks", "task", "peek", *args],
                                capture_output=True, text=True, check=False)
    except OSError:
        print("task-docs focused: niritasks not found on PATH", file=sys.stderr)
        return False
    if result.returncode != 0:
        print(f"task-docs focused: {result.stderr.strip()[-300:]}", file=sys.stderr)
    return result.returncode == 0


def focused_action():
    """The pane, tab and workspace focus hooks: peek at the focused pane's task,
    or at nothing when it runs none. The three events one switch can fire find
    the same pane and peek the same way; nothing is remembered here, because
    the daemon's peek lives in its memory and ignores a peek it already shows.
    A task niritasks does not know, one whose branch outlived it, peeks at
    nothing."""
    found = focused_pane()
    if not found:
        return
    pane, cwd = found
    uuid8 = find_task(pane, cwd)
    if not (uuid8 and peek(uuid8)):
        peek("--clear")


def parse_docs(stdout):
    """`niritasks task docs` lines as (label, path) pairs; a line with no tab is skipped."""
    return [tuple(line.split("\t", 1)) for line in stdout.splitlines() if "\t" in line]


def docs(uuid8, cwd):
    """The task's documents, from niritasks; its notes written into the state folder."""
    result = subprocess.run(
        ["niritasks", "task", "docs", uuid8, "--project", str(cwd), "--notes-dir", str(state_dir())],
        capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip()[-300:] or "niritasks task docs failed")
    return parse_docs(result.stdout)


def choose(items):
    """fzf over the labels, with a preview; the picked path, or None on Esc."""
    result = subprocess.run(
        ["fzf", "--delimiter", "\t", "--with-nth", "1", "--no-sort", "--prompt", "doc> ",
         "--header", "Enter annotates it beside the task's Claude, Esc cancels",
         "--preview", PREVIEW, "--preview-window", "right,60%,wrap"],
        input="\n".join(f"{label}\t{path}" for label, path in items),
        stdout=subprocess.PIPE, text=True, check=False,
    )
    choice = result.stdout.strip()
    if result.returncode != 0 or "\t" not in choice:
        return None
    return choice.split("\t", 1)[1]


def open_doc(path, claude):
    """Markdown in plannotator-tui, right of the task's Claude and delivering to it;
    anything else, a report with no twin, in the browser."""
    if not path.endswith(".md"):
        # setsid -f detaches it into its own session: the overlay closes when pick()
        # returns, and a hangup then must not reach the browser.
        subprocess.run(["setsid", "-f", "xdg-open", path], stdin=subprocess.DEVNULL,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        return
    result = herdr("plugin", "pane", "open", "--plugin", PLUGIN, "--entrypoint", "review",
                   "--placement", "split", "--direction", "right", "--target-pane", claude, "--focus",
                   "--cwd", str(Path(path).parent),
                   "--env", f"PLANNOTATOR_TUI_FILE={path}", "--env", f"PLANNOTATOR_TUI_DELIVER_TO={claude}")
    if result.returncode != 0:
        notify(f"Task docs: could not open the reviewer. {result.stderr.strip()[-300:]}")


def pick():
    """Run in the overlay: list the task's documents and open the one picked."""
    uuid8 = os.environ.get("TASK_DOCS_TASK")
    claude = os.environ.get("TASK_DOCS_CLAUDE")
    if not uuid8 or not claude:
        notify("Task docs: the list was opened without a task.")
        return
    try:
        items = docs(uuid8, Path.cwd())
    except (OSError, RuntimeError) as error:
        notify(f"Task docs: could not list the documents. {error}")
        return
    if not items:
        notify(f"Task docs: task {uuid8} has no report, notes or plan to annotate.")
        return
    try:
        path = choose(items)
    except OSError:
        notify("Task docs: fzf not found. Install fzf to pick a document.")
        return
    if path:
        open_doc(path, claude)


def review():
    """Run in the review pane: plannotator-tui's pane mode, which reads
    PLANNOTATOR_TUI_FILE and PLANNOTATOR_TUI_DELIVER_TO."""
    binary = reviewer()
    if not binary:
        print(f"plannotator-tui is not installed. {FULL}\n\npress enter to close", file=sys.stderr)
        notify(f"Task docs: plannotator-tui is needed. {FULL}")
        input()
        return
    os.execv(binary, [binary, "herdr", "pane"])


def main(argv):
    commands = {"open": open_action, "panel": panel_action, "focused": focused_action,
                "pick": pick, "review": review}
    if len(argv) != 2 or argv[1] not in commands:
        print("usage: task_docs.py open|panel|focused|pick|review", file=sys.stderr)
        return 2
    commands[argv[1]]()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
