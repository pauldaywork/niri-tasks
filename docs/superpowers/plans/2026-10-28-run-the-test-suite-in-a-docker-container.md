# Run the Test Suite in a Docker Container Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `bash tests/container.sh` runs all five suites against the current checkout, uncommitted edits included, inside a Docker container, in the foreground or the background, and touches nothing on the machine: not its niri, workspaces, task database or installed `niritasks`.

**Architecture:** An image built from `ubuntu:26.04` (`tests/container/Dockerfile`) holds every prerequisite `tests/all.sh` checks for. The wrapper `tests/container.sh` builds that image when its Dockerfile has changed, streams a tar snapshot of the working tree into a fresh container on stdin (no bind mounts at all), and tees the container's output to a log under `target/container-tests/`. Inside, `tests/container/inside.sh` starts a headless sway on software rendering, runs an outer niri as its one window with two named workspaces, builds the snapshot with `NIRITASKS` pointing at that build, and runs `tests/all.sh` unchanged — which nests its own niris inside the outer one exactly as it does on the desktop.

**Tech Stack:** Bash; Docker 29.1 (on the machine, user in the `docker` group); Ubuntu 26.04 with niri 26.04 from the `avengemedia/danklinux` PPA, sway 1.11 (wlroots headless + pixman), Mesa 26.0 llvmpipe, Rust 1.95.0 via rustup-init 1.29.2, Taskwarrior 3.5.0 built from its release tarball, Iosevka Term 34.8.1.

**Spec:** Taskwarrior task `f60e243e-83fa-45ef-9705-dc94cc3c0f0b` — its description and notes (`task rc.json.array=on f60e243e export`). No separate spec file.

## Global Constraints

- A Docker container, not a VM. A VM only if the spike (Task 1) proves niri cannot run in a container — then **stop and ask** before doing anything else.
- Image from `ubuntu:26.04` with niri 26.04, wtype, python3-pil, flock, git, the Rust toolchain and Taskwarrior 3.5.0, all baked in, so a run installs nothing.
- Code goes in as a snapshot of the current working tree, uncommitted edits included, copied in at start; it builds there with `NIRITASKS` set to that build; the cargo cache lives in a Docker volume.
- Inside, a headless compositor runs an outer niri with two named workspaces, and `tests/all.sh` runs as it does now, nesting its niri inside that one. `tests/all.sh` and the e2e scripts are not rewritten for the container.
- The wrapper is `bash tests/container.sh`: builds the image if needed, copies the tree in, runs `tests/all.sh`, saves the log, and can run in the background.
- Nothing on the host is touched: no bind mount of any host path, no host `XDG_RUNTIME_DIR`, Wayland or niri socket, no host network namespace, no `--privileged`. The only host writes are the log under the checkout's own `target/` and Docker's own image and volumes.
- Done when: from any worktree the wrapper runs all five suites in the container with zero skips and the same results as on the host, the host's windows, workspaces and task database are unchanged, and README documents it beside `tests/all.sh`.
- Out of scope: CI, and replacing `tests/all.sh` or `try.sh` on the host.
- Commits: Conventional Commits with a scope, `test(container): …` or `docs(readme): …`, subject ≤72 characters, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` trailer.

## Facts checked on 2026-10-28 (so you need not)

- **niri is not in Ubuntu 26.04's own archive** (`apt-cache policy niri` in `ubuntu:26.04` is empty), contrary to the task's "apt's niri". The machine's `niri 26.04ppa3` comes from `ppa:avengemedia/danklinux`; `add-apt-repository -y ppa:avengemedia/danklinux` in the container installs the same package.
- In `ubuntu:26.04` apt has: sway 1.11, weston 14.0.2, Mesa 26.0.8 (`libgl1-mesa-dri`, `libegl-mesa0`), python3 3.14, python3-pil 12.1, wtype 0.4, alacritty 0.16.1, cmake 4.2.3, g++ 15.2, uuid-dev, libgtk-4-dev 4.22.4, libgtk4-layer-shell-dev 1.3.0, dbus-daemon, adwaita-icon-theme, cargo 1.93 (not used: see rustup below). Taskwarrior is only 2.6.2 — too old.
- **Spike, half done:** in that image, `sway` with `WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1` starts; `niri` started on it with `LIBGL_ALWAYS_SOFTWARE=1` runs (winit backend, output "winit" 1916x1053 under sway's default 1920x1080 output), `niri msg workspaces` lists the two named workspaces from its config, the first focused. A nested niri started on that outer niri (the way `tests/lib/nested-niri.sh` does it) comes up, and `niri msg action screenshot-screen` on it writes a PNG whose pixels are the configured `#406080` background. Log noise that is harmless: `EGL_EXT_device_drm` not supported, no login1/locale1 on D-Bus, no xcursor theme, no Xwayland. Not yet shown: the panel and box GTK clients drawing, and the pixel checks passing — that is Task 1.
- Taskwarrior 3.5.0 tarball: `https://github.com/GothenburgBitFactory/taskwarrior/releases/download/v3.5.0/task-3.5.0.tar.gz`, sha256 `9ea64b411f8314414f440ec765dfdf5a86c9f6159df47e2f60cff3db6b31157a`. Needs CMake within `3.24...4.0` policy range (4.2 is fine, the machine built it with 4.3), C++17, libuuid, Rust ≥ 1.91.1 (pinned in `src/taskchampion-cpp/Cargo.toml`), network at configure. It must be **installed** with `cmake --install`, not run from the build tree, or it cannot find `default.theme` (upstream #4174).
- rustup-init 1.29.2: `https://static.rust-lang.org/rustup/archive/1.29.2/x86_64-unknown-linux-gnu/rustup-init`, sha256 `5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4`. The machine builds with rustc 1.95.0.
- The panel and box fonts are hard-coded: `10pt "Iosevka Term Extended"` (`src/panel/style.rs:12`, `src/taskbox/style.rs`). The machine has Iosevka Term **34.8.1** in `~/.local/share/fonts/iosevka-term/` (54 TTFs), not from apt. `https://github.com/be5invis/Iosevka/releases/download/v34.8.1/PkgTTF-IosevkaTerm-34.8.1.zip`, sha256 `c9929ef8c27e5469906468e378e11d3023690ae9cd411a4af3de78190b901075`, holds the same 54 files. apt's `fonts-iosevka` is not the Term Extended family.
- Skips that only a missing thing causes, so the container must provide it for "zero skips": `e2e-box.sh` skips the refocus check without `alacritty`; `e2e-panel.sh` skips the project-list checks unless `$HOME/Projects` holds a folder not named `e2e` (`tests/e2e-panel.sh:1008`, `:1056`); `e2e-panel.sh` skips Ctrl+Enter refining when `herdr` is on `NESTED_SPAWN_PATH` (`:734`) — never in the container, which has no herdr.
- Other tools the suites call: `sqlite3` (`tests/backup.rs:143`), `tar`, `git` (`herdr/task-docs/tests/test_task_docs.py:115`), `timeout`, `realpath`, `flock`, `systemctl --user` (absent is fine: `tests/lib/nested-niri.sh:82` falls back to `0` both before and after).
- The worktree's `.git` is a file pointing at the main repo, so the snapshot is the file list from `git ls-files --cached --others --exclude-standard`, not a copy of `.git`. No suite needs the checkout itself to be a git repository (each git test makes its own).
- `tests/all.sh` defaults `NIRITASKS` to the installed `niritasks`; in a worktree it must be set (see memory: "all.sh needs NIRITASKS= in a worktree"). The host run for comparison is `NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh` after `cargo build`.

## File Structure

- Create `tests/container/Dockerfile` — the image: every prerequisite, pinned, nothing fetched at run time except crates on a cold cargo cache.
- Create `tests/container/sway.conf` — the headless compositor's config: one large output, no borders, no Xwayland.
- Create `tests/container/niri.kdl` — the outer niri's config: two named workspaces, and a startup line that tells `inside.sh` its sockets.
- Create `tests/container/inside.sh` — what runs in the container: the desktop `tests/all.sh` expects, the build, then `tests/all.sh` (or a command given instead).
- Create `tests/container.sh` — the wrapper on the host: image, volumes, snapshot, run, log.
- Modify `README.md` — "Running the tests": the command beside `tests/all.sh`, a row in the Needs/Touches table, and a short subsection on what it does and costs.

The container files go in `tests/container/` (a directory the snapshot carries in, so `inside.sh` edits need no image rebuild); the wrapper sits beside `tests/all.sh` because that is the command people type.

---

### Task 1: The image, the outer niri, and the spike gate on e2e-panel.sh

The spike the task asks for, made with the real files rather than throwaway ones: if the panel's pixel checks pass here, the rest is plumbing. **If niri itself cannot run, or the panel's checks cannot be made to pass under software rendering with the remedies in Step 5, stop and ask the user** — do not move to a VM, and do not weaken a check.

**Files:**
- Create: `tests/container/Dockerfile`
- Create: `tests/container/sway.conf`
- Create: `tests/container/niri.kdl`
- Create: `tests/container/inside.sh`

**Interfaces:**
- Consumes: nothing from other tasks. `tests/all.sh`, `tests/e2e-*.sh` and `tests/lib/nested-niri.sh` as they are.
- Produces: an image buildable with `docker build -t <tag> tests/container`; `bash tests/container/inside.sh [command...]`, run from `/work` (the snapshot) as root inside that image under `dbus-run-session`, which exits with the command's status (default `bash tests/all.sh`), or 1 with a `container: …` message on stderr when its desktop or build cannot start. It expects the cargo build directory at `/work/target` and the cargo registry at `/usr/local/cargo/registry` (both volumes in Task 2).

- [ ] **Step 1: Write the Dockerfile**

`tests/container/Dockerfile`:

```dockerfile
# The image tests/container.sh runs the test suites in. Everything
# tests/all.sh checks for is baked in, so a run installs nothing; the only
# thing fetched at run time is crates, into a cargo cache that outlives the
# container. tests/container.sh names the image after this file's hash, so
# any edit here builds a new image on the next run.

FROM ubuntu:26.04 AS taskwarrior
# Taskwarrior 3.5.0, the version the suites are written against. apt has
# only 2.6.2. Built in a stage of its own so the image carries the binary,
# not cmake's build tree. Installed, not run from the build tree: 3.5.0
# cannot find default.theme otherwise (upstream #4174).
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl git build-essential cmake uuid-dev libsqlite3-dev cargo \
 && rm -rf /var/lib/apt/lists/*
RUN curl -fsSLo /tmp/task.tar.gz https://github.com/GothenburgBitFactory/taskwarrior/releases/download/v3.5.0/task-3.5.0.tar.gz \
 && echo "9ea64b411f8314414f440ec765dfdf5a86c9f6159df47e2f60cff3db6b31157a  /tmp/task.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/task.tar.gz -C /tmp \
 && cmake -S /tmp/task-3.5.0 -B /tmp/task-build -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/opt/task \
 && cmake --build /tmp/task-build -j"$(nproc)" \
 && cmake --install /tmp/task-build

FROM ubuntu:26.04
ENV DEBIAN_FRONTEND=noninteractive LANG=C.UTF-8 \
    RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/opt/task/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
# niri 26.04 is not in Ubuntu's archive: the machine's comes from the
# avengemedia/danklinux PPA, and so does this one. sway is only the
# headless compositor the outer niri runs in (inside.sh); Mesa is its
# software renderer. The rest is what tests/all.sh checks for, what the
# crate links against, and what the suites call along the way.
RUN apt-get update \
 && apt-get install -y --no-install-recommends software-properties-common gpg-agent ca-certificates \
 && add-apt-repository -y ppa:avengemedia/danklinux \
 && apt-get install -y --no-install-recommends \
      niri sway libgl1-mesa-dri libegl-mesa0 libegl1 dbus-daemon \
      wtype alacritty python3 python3-pil git util-linux procps sqlite3 \
      curl unzip build-essential pkg-config libgtk-4-dev libgtk4-layer-shell-dev \
      libuuid1 libsqlite3-0 fontconfig adwaita-icon-theme \
 && rm -rf /var/lib/apt/lists/*
COPY --from=taskwarrior /opt/task /opt/task
# The machine's compiler, so a build here is the build there.
RUN curl -fsSLo /tmp/rustup-init https://static.rust-lang.org/rustup/archive/1.29.2/x86_64-unknown-linux-gnu/rustup-init \
 && echo "5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.95.0 \
 && rm /tmp/rustup-init
# The panel and the box draw in Iosevka Term Extended, by name
# (src/panel/style.rs). Without it GTK falls back to another font and every
# text width the panel test measures moves. The machine's version.
RUN curl -fsSLo /tmp/iosevka.zip https://github.com/be5invis/Iosevka/releases/download/v34.8.1/PkgTTF-IosevkaTerm-34.8.1.zip \
 && echo "c9929ef8c27e5469906468e378e11d3023690ae9cd411a4af3de78190b901075  /tmp/iosevka.zip" | sha256sum -c - \
 && mkdir -p /usr/local/share/fonts/iosevka-term \
 && unzip -q /tmp/iosevka.zip -d /usr/local/share/fonts/iosevka-term \
 && rm /tmp/iosevka.zip \
 && fc-cache -f
# A taskrc, so a bare `task` never stops to ask whether to make one. Every
# suite points TASKRC at a sandbox of its own anyway.
RUN printf 'data.location=/root/.task\n' > /root/.taskrc
WORKDIR /work
```

Why `cargo` from apt in the first stage only: Taskwarrior needs Rust ≥ 1.91.1 and apt's 1.93 meets it, without fetching rustup twice.

- [ ] **Step 2: Write the compositor and outer niri configs**

`tests/container/sway.conf`:

```
# The headless compositor the container's outer niri runs in
# (tests/container/inside.sh). niri needs one: its own headless backend is
# only for niri's tests. One output big enough that the nested niri the e2e
# tests park, 1600x1000, fits on the outer niri's with room to spare.
output * resolution 2560x1440
default_border none
gaps inner 0
xwayland disable
```

`tests/container/niri.kdl`:

```kdl
// The outer niri in the test container (tests/container/inside.sh): the
// niri tests/all.sh takes for yours. Two named workspaces, the first
// focused, as tests/e2e-tag.sh wants; the e2e tests park their nested niri
// on the empty one niri keeps after them. The startup line is how
// inside.sh learns this niri's sockets.
hotkey-overlay { skip-at-startup; }
xwayland-satellite { off; }
animations { off; }
workspace "alpha"
workspace "beta"
spawn-sh-at-startup "env > $XDG_RUNTIME_DIR/outer.env.tmp && mv $XDG_RUNTIME_DIR/outer.env.tmp $XDG_RUNTIME_DIR/outer.env"
```

- [ ] **Step 3: Write inside.sh**

`tests/container/inside.sh`:

```bash
#!/usr/bin/env bash
# What runs inside the test container: the far side of tests/container.sh,
# run from the snapshot of the checkout in /work.
#
#   bash tests/container/inside.sh                          # tests/all.sh
#   bash tests/container/inside.sh bash tests/e2e-box.sh    # one suite
#
# It stands up the desktop tests/all.sh expects to find on a machine, then
# gets out of the way: a headless sway on software rendering, an outer niri
# as its one window with two named workspaces, and ~/Projects with folders
# in it. tests/all.sh then runs as it does on the desktop, and its e2e
# suites nest their own niri inside this one. Nothing here is the machine's:
# no socket, file or display of the host reaches the container.
#
# It builds the snapshot first and points NIRITASKS at that build, so no
# suite can test an installed binary: there is none in here.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

die() { echo "container: $*" >&2; exit 1; }

export XDG_RUNTIME_DIR=/tmp/runtime
mkdir -p -m 700 "$XDG_RUNTIME_DIR"
# Mesa's llvmpipe for every GL client: the outer niri, the nested niris and
# GTK. There is no GPU in here.
export LIBGL_ALWAYS_SOFTWARE=1

WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 \
    sway -c tests/container/sway.conf >/tmp/sway.log 2>&1 &
sway_display=""
for _ in $(seq 1 50); do
    sway_display=$(cd "$XDG_RUNTIME_DIR" && ls -d wayland-[0-9] 2>/dev/null | head -n 1)
    [ -n "$sway_display" ] && [ -S "$XDG_RUNTIME_DIR/$sway_display" ] && break
    sleep 0.2
done
[ -n "$sway_display" ] || die "sway did not start:
$(tail -n 20 /tmp/sway.log)"

WAYLAND_DISPLAY="$sway_display" niri -c tests/container/niri.kdl >/tmp/niri.log 2>&1 &
for _ in $(seq 1 50); do
    [ -s "$XDG_RUNTIME_DIR/outer.env" ] && break
    sleep 0.2
done
[ -s "$XDG_RUNTIME_DIR/outer.env" ] || die "the outer niri did not start:
$(tail -n 20 /tmp/niri.log)"
NIRI_SOCKET=$(sed -n 's/^NIRI_SOCKET=//p' "$XDG_RUNTIME_DIR/outer.env")
WAYLAND_DISPLAY=$(sed -n 's/^WAYLAND_DISPLAY=//p' "$XDG_RUNTIME_DIR/outer.env")
export NIRI_SOCKET WAYLAND_DISPLAY
[ -n "$NIRI_SOCKET" ] && [ -n "$WAYLAND_DISPLAY" ] ||
    die "the outer niri gave its programs no NIRI_SOCKET or WAYLAND_DISPLAY"
niri msg workspaces || die "the outer niri does not answer"

# The panel test moves a task to a project folder and opens the project
# list; both need a ~/Projects with a folder in it, or they are skipped.
mkdir -p "$HOME/Projects/alpha" "$HOME/Projects/beta"

echo "building $(pwd)"
cargo build --locked || die "cargo build failed"
export NIRITASKS="$PWD/target/debug/niritasks"

[ $# -gt 0 ] || set -- bash tests/all.sh
"$@"
```

- [ ] **Step 4: Build the image and run e2e-panel.sh in it (the spike)**

Run from the worktree root. The volumes are the ones Task 2's wrapper uses, so this build is not wasted:

```bash
docker build -t niritasks-tests:spike tests/container
docker volume create --label niritasks.tests=1 niritasks-tests-cargo >/dev/null
docker volume create --label niritasks.tests=1 niritasks-tests-target-spike >/dev/null
git ls-files -z --cached --others --exclude-standard |
    while IFS= read -r -d '' f; do { [ -e "$f" ] || [ -L "$f" ]; } && printf '%s\0' "$f"; done |
    tar --null -T - -cf - |
    docker run -i --init --name niritasks-spike \
        -v niritasks-tests-target-spike:/work/target -v niritasks-tests-cargo:/usr/local/cargo/registry \
        -e NIRITASKS_E2E_KEEP=1 niritasks-tests:spike \
        bash -c 'tar -xf - -C /work && exec dbus-run-session -- bash /work/tests/container/inside.sh bash tests/e2e-panel.sh'
echo "exit: $?"
```

The first build takes several minutes (Taskwarrior, then the crate). Expected on success: the outer niri's `niri msg workspaces` listing `alpha` (focused) and `beta`, cargo's build, then e2e-panel.sh's own `PASS` lines ending `passed: N   failed: 0   skipped: 0`, and `exit: 0`.

No `--rm` here on purpose: the container stays so its files can be read. With `NIRITASKS_E2E_KEEP=1` the run prints `the run's files are kept in /tmp/tmp.XXXX`; copy them out with:

```bash
mkdir -p target/container-tests/spike
docker cp niritasks-spike:/tmp/tmp.XXXX target/container-tests/spike/run
docker cp niritasks-spike:/tmp/niri.log target/container-tests/spike/outer-niri.log
docker rm niritasks-spike
```

and look at the frames (`target/container-tests/spike/run/shots/*.png`) with the Read tool.

- [ ] **Step 5: Make e2e-panel.sh pass in the container, or stop**

If Step 4 passed with zero failures and zero skips, go to Step 6. Otherwise diagnose from the kept frames and logs, one cause at a time, rerunning Step 4 (after `docker rm niritasks-spike`) after each change. Fixes go in `tests/container/` only — never in `tests/e2e-panel.sh`, `tests/lib/nested-niri.sh` or `src/`. The likely causes, in the order to try:

1. **The outer output is too small** (`the nested niri's output is …, not 1600x1000`, or the window could not be parked): check `niri msg outputs` in the outer niri's log section; if `output * resolution` did not apply in headless sway, use `output HEADLESS-1 mode --custom 2560x1440` instead.
2. **GTK draws nothing or crashes** (`the daemon exited at once`, black frames): GTK 4.22's default GL renderer on llvmpipe. Add `export GSK_RENDERER=cairo` beside `LIBGL_ALWAYS_SOFTWARE` in `inside.sh`, and say why in a comment. Only if that is not enough, try `GSK_RENDERER=vulkan` with `mesa-vulkan-drivers` (lavapipe) added to the image.
3. **No session bus** (GLib `Failed to register` / `Cannot autolaunch D-Bus`): the run is already under `dbus-run-session`; check `dbus-daemon` is in the image.
4. **Text widths differ** (an edge or column off by a few pixels from what the test expects): check `fc-match "Iosevka Term Extended"` in the container names `IosevkaTerm-Extended.ttf`; if not, `fc-cache` did not see the fonts.
5. **Timeouts** (`never held still`, `ten frames, no two alike`, or a slide that does not finish): llvmpipe is slower than the GPU. Check `nproc` inside the container; give the container more CPUs if Docker limits them. Do not lengthen a test's waits — report it to the user instead.

**Stop and ask the user** — with the log and a frame — if the outer niri does not start, if the nested niri cannot be parked, or if a check still fails after the remedies above. That is the task's "spike proves niri cannot run in a container" gate; a VM is the user's call, not this plan's.

- [ ] **Step 6: Clean up the spike and commit**

```bash
docker rm -f niritasks-spike 2>/dev/null
docker volume rm niritasks-tests-target-spike
docker image rm niritasks-tests:spike
chmod +x tests/container/inside.sh
git add tests/container
git commit -m "test(container): run the suites' desktop in a Docker image

A ubuntu:26.04 image with every prerequisite tests/all.sh checks for,
and the script that stands up a headless sway and an outer niri with
two named workspaces inside it before running the suites. e2e-panel.sh
passes in it with no skips under software rendering.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If Step 5 needed any remedy, name it in the commit body (e.g. "GTK draws with GSK_RENDERER=cairo: its GL renderer …").

---

### Task 2: The wrapper, tests/container.sh, and its README

**Files:**
- Create: `tests/container.sh`
- Modify: `README.md` — the "Running the tests" section (around line 592–622) and a new subsection after the e2e-box paragraph, before "> [!IMPORTANT]" (around line 675)

**Interfaces:**
- Consumes: `tests/container/Dockerfile` (built with `docker build tests/container`), and `bash /work/tests/container/inside.sh [command...]` from Task 1, run under `dbus-run-session` with `/work/target` and `/usr/local/cargo/registry` mounted.
- Produces: `bash tests/container.sh [command...]` — exits with the in-container command's status (default `bash tests/all.sh`); 1 when Docker is missing or unreachable, the image cannot be built, or a run of the same checkout is already going. Writes the whole run to `target/container-tests/<UTC stamp>.log` and points `target/container-tests/latest.log` at it. Containers carry the label `niritasks.tests.root=<checkout path>`; volumes carry `niritasks.tests=1`.

- [ ] **Step 1: Run the command that should not exist yet**

Run: `bash tests/container.sh; echo "exit: $?"`
Expected: `bash: tests/container.sh: No such file or directory`, `exit: 127`.

- [ ] **Step 2: Write tests/container.sh**

```bash
#!/usr/bin/env bash
# Every test suite, in a Docker container: tests/all.sh against this
# checkout, uncommitted edits included, with nothing on the machine touched.
#
#   bash tests/container.sh                          # tests/all.sh
#   bash tests/container.sh bash tests/e2e-box.sh    # one suite
#   bash tests/container.sh >/dev/null 2>&1 &        # in the background
#
# tests/all.sh on the desktop nests its niris on your niri, names one of
# your workspaces and, unless told otherwise, runs the installed niritasks.
# This runs the same tests/all.sh, unchanged, against a desktop of the
# container's own (tests/container/inside.sh): a headless sway, an outer
# niri with two named workspaces in it, and a build of this checkout. So
# your niri, your workspaces, your task database and your niritasks are
# never in reach — the container gets no mount, socket or display of the
# machine's, only the files of this checkout, copied in on stdin.
#
# The copy is what git would see of the working tree: tracked files as they
# are now, edited or not, and untracked ones that are not ignored. Not
# target/, not .git.
#
# The image (tests/container/Dockerfile) is built the first time, and again
# only when the Dockerfile changes: it is tagged with the file's hash. The
# cargo registry lives in one Docker volume shared by every checkout, and
# each checkout's build directory in a volume of its own, so a second run
# rebuilds only what changed. Two runs of the same checkout at once would
# share that build directory, so the second refuses; runs of different
# worktrees go side by side.
#
# Everything the run prints is also saved, in target/container-tests/, with
# latest.log pointing at the newest. It exits with tests/all.sh's status:
# non-zero only when a suite that ran said no.
set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || exit 1
cd "$ROOT" || exit 1

command -v docker >/dev/null || { echo "docker is required" >&2; exit 1; }
docker info >/dev/null 2>&1 || { echo "docker is not running, or this user cannot reach it" >&2; exit 1; }

IMAGE="niritasks-tests:$(sha256sum tests/container/Dockerfile | cut -c1-12)"
KEY=$(printf '%s' "$ROOT" | sha256sum | cut -c1-12)
TARGET_VOLUME="niritasks-tests-target-$KEY"
REGISTRY_VOLUME="niritasks-tests-cargo"
NAME="niritasks-tests-$KEY-$$"

if [ -n "$(docker ps -q --filter "label=niritasks.tests.root=$ROOT")" ]; then
    echo "a container run of $ROOT is already going, and two would share one build directory" >&2
    exit 1
fi

LOGS="$ROOT/target/container-tests"
mkdir -p "$LOGS" || exit 1
LOG="$LOGS/$(date -u +%Y%m%dT%H%M%SZ)-$$.log"
ln -sfn "$(basename "$LOG")" "$LOGS/latest.log"
exec > >(tee "$LOG") 2>&1
echo "repo  $ROOT"
echo "log   $LOG"

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
    echo "building $IMAGE: once, and again only when tests/container/Dockerfile changes"
    docker build -t "$IMAGE" tests/container || exit 1
fi
for volume in "$TARGET_VOLUME" "$REGISTRY_VOLUME"; do
    docker volume inspect "$volume" >/dev/null 2>&1 ||
        docker volume create --label niritasks.tests=1 "$volume" >/dev/null || exit 1
done

# The working tree as git sees it, NUL-separated. A tracked file deleted but
# not yet committed is left out rather than failing the copy.
snapshot() {
    git ls-files -z --cached --others --exclude-standard |
        while IFS= read -r -d '' f; do
            { [ -e "$f" ] || [ -L "$f" ]; } && printf '%s\0' "$f"
        done |
        tar --null -T - -cf -
}

# Interrupted, the container goes too, rather than run on unseen. The run
# is waited for in the background because bash runs a trap only once the
# foreground command is over, and docker run is not over until the suites
# are.
trap 'docker rm -f "$NAME" >/dev/null 2>&1' EXIT
trap 'exit 130' INT TERM

snapshot | docker run --rm -i --init --name "$NAME" \
    --label "niritasks.tests.root=$ROOT" \
    -v "$TARGET_VOLUME:/work/target" \
    -v "$REGISTRY_VOLUME:/usr/local/cargo/registry" \
    "$IMAGE" \
    bash -c 'tar -xf - -C /work && exec dbus-run-session -- bash /work/tests/container/inside.sh "$@"' inside "$@" &
wait "$!"
status=$?

echo
echo "log   $LOG"
exit "$status"
```

Notes for the implementer:
- No `-t`: a TTY would stop it running in the background or from an agent's shell, and would mangle the tar on stdin.
- `wait "$!"` rather than a foreground pipeline: see the comment above the traps. `$!` is the pipeline's last command, `docker run`, so its status is the run's.
- `"$@"` after `inside` passes the optional command through `bash -c` intact (`inside` becomes `$0`).
- If Task 1 needed an extra environment variable (e.g. `GSK_RENDERER`), it lives in `inside.sh`, not here.

- [ ] **Step 3: Run one quick suite through it**

Run: `chmod +x tests/container.sh && bash tests/container.sh bash tests/e2e-tag.sh; echo "exit: $?"`
Expected: `repo` and `log` lines, the image build (first time only), cargo's build, e2e-tag.sh's `PASS` lines with no `FAIL`, the closing `log` line, `exit: 0`. Then:

Run: `ls -l target/container-tests/ && tail -n 3 target/container-tests/latest.log`
Expected: a `<stamp>-<pid>.log` file, `latest.log -> <that file>`, and the log ends with the same lines the terminal showed.

If e2e-tag.sh fails, read its message: it is the first suite to use the outer niri's two workspaces. A fix goes in `tests/container/niri.kdl` or `inside.sh`.

- [ ] **Step 4: Check the refusals**

Run, in two steps:

```bash
bash tests/container.sh bash -c 'sleep 60' >/dev/null 2>&1 &
sleep 15; bash tests/container.sh true; echo "exit: $?"
```

Expected: `a container run of /home/…/task-test-run-the-test-suite-in-a-docker-f60e243e is already going, and two would share one build directory`, `exit: 1`. Then `wait` for the background run.

Run: `bash tests/container.sh false; echo "exit: $?"`
Expected: `exit: 1` — the in-container command's status comes back out.

Run: `bash tests/container.sh bash -c 'sleep 300' >/dev/null 2>&1 & pid=$!; sleep 20; kill -TERM "$pid"; wait "$pid"; echo "exit: $?"; docker ps --filter label=niritasks.tests.root="$PWD" -q | wc -l`
Expected: `exit: 130` and `0` — a stopped run takes its container with it, at once rather than after the 300s. (TERM, not INT: a background job in a non-interactive shell ignores INT. Ctrl-C on a foreground run reaches the same trap.)

- [ ] **Step 5: Document it in README.md**

In "### Running the tests", add the command to the code block, under `tests/all.sh`:

```bash
bash tests/all.sh           # everything this machine can run       ~100s
bash tests/container.sh     # everything, in a Docker container     ~<measured>s

cargo test                  # unit and write-path                   ~2s
```

Replace `~<measured>s` with the warm-cache time from Task 3 Step 2 (fill it in then; leave this line for that step).

Add a row to the Needs/Touches table, directly under the `tests/all.sh` row:

```markdown
| `tests/container.sh` | Docker, and the user in the `docker` group. The first run builds the image (several minutes) and needs the network | Nothing of yours. Docker's own image and volumes (`niritasks-tests*`), and its log in `target/container-tests/` |
```

Add a subsection after the paragraph that ends "…and a long note list scrolls, keeping a row made by Enter at the bottom in view." and before the `> [!IMPORTANT]` box:

````markdown
### Running the tests in a container

`tests/container.sh` runs `tests/all.sh` in a Docker container, against this
checkout as it stands, uncommitted edits included. Nothing of your desktop is
in reach: the container gets no mount, socket or display of yours, only a copy
of the files git would see, so your niri, workspaces, task database and
installed `niritasks` are never touched, and there is no workspace to keep off.
Run it from any worktree, side by side with other worktrees' runs, in the
foreground or in the background:

```bash
bash tests/container.sh                          # tests/all.sh
bash tests/container.sh bash tests/e2e-box.sh    # one suite
bash tests/container.sh >/dev/null 2>&1 &        # in the background
tail -f target/container-tests/latest.log
```

Inside, `tests/container/inside.sh` starts a headless sway on software
rendering, an outer niri in it with two named workspaces, builds the copy, and
runs `tests/all.sh` with `NIRITASKS` set to that build; the e2e suites nest
their niri in the outer one as they would in yours. The image
(`tests/container/Dockerfile`) is built on the first run and again only when
the Dockerfile changes; cargo's registry and each checkout's build directory
live in Docker volumes, so later runs rebuild only what changed. A second run
of the same checkout while one is going refuses, since the two would share a
build directory.

Every run is logged in `target/container-tests/`, `latest.log` the newest.
It exits as `tests/all.sh` does. To reclaim the space:

```bash
docker volume rm $(docker volume ls -q --filter label=niritasks.tests=1)
docker image rm $(docker image ls -q niritasks-tests)
```
````

- [ ] **Step 6: Commit**

```bash
git add tests/container.sh README.md
git commit -m "test(container): add tests/container.sh to run the suites in Docker

Builds the image when its Dockerfile changes, copies the working tree in
on stdin, runs tests/all.sh there and keeps the log in
target/container-tests/. No mount or socket of the machine reaches the
container, so it runs beside your desktop with nothing to keep off.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Prove the done-when: all five suites, zero skips, same results, host untouched

**Files:**
- Modify (only if a suite fails or skips in the container): `tests/container/Dockerfile`, `tests/container/inside.sh`, `tests/container/niri.kdl`, `tests/container/sway.conf`
- Modify: `README.md` — the measured time in the code block from Task 2 Step 5

**Interfaces:**
- Consumes: `bash tests/container.sh` from Task 2.
- Produces: nothing new for code; evidence in the final report (the two summaries and the host-state diff).

- [ ] **Step 1: Record the host's state before**

```bash
mkdir -p target/container-tests/proof
cd target/container-tests/proof
niri msg -j workspaces | python3 -c 'import json,sys; [print(w["id"], w["idx"], w["output"], w.get("name"), w["is_focused"]) for w in sorted(json.load(sys.stdin), key=lambda w: w["id"])]' > workspaces.before
niri msg -j windows | python3 -c 'import json,sys; [print(w["id"], w["workspace_id"], w["app_id"]) for w in sorted(json.load(sys.stdin), key=lambda w: w["id"])]' > windows.before
task rc.hooks=0 rc.gc=off rc.json.array=on export 2>/dev/null | sha256sum > tasks.before
ls -l --time-style=full-iso "$(task _get rc.data.location)" > taskdata.before
readlink -f "$(command -v niritasks)" > niritasks.before; sha256sum "$(cat niritasks.before)" >> niritasks.before
systemctl --user show -p MainPID --value niri-tasks.service > service.before
cd -
```

Tell the user before Step 2 that you are about to run the whole suite in the container, and ask them not to add, edit or complete tasks, nor open or close windows, until it finishes — otherwise the before/after comparison proves nothing.

- [ ] **Step 2: Run everything in the container**

Run: `time bash tests/container.sh; echo "exit: $?"`
Expected: five suites with `PASS`, none `SKIP`, none `FAIL`, ending `5 passed   0 failed   0 skipped` and `exit: 0`. Each suite's own inner summary (`passed: N   failed: 0   skipped: 0`) shows zero skips too. Note the wall time of this warm-cache run (run it a second time if this one built the image) and put it in the README line from Task 2 Step 5, rounded like its neighbours (`~NNNs`).

If a suite fails or skips, fix the container side, the same way as Task 1 Step 5. The wrapper's container is removed when it ends, so to see a failing e2e suite's frames, rerun Task 1 Step 4's named-container command (without `--rm`, with `NIRITASKS_E2E_KEEP=1`) with that suite in place of `tests/e2e-panel.sh`, using the wrapper's image tag (`niritasks-tests:<hash>`, printed by `docker image ls niritasks-tests`). Never change a test or `src/` to make the container pass; if a check can only fail in the container, stop and tell the user which and why.

- [ ] **Step 3: Check the host is unchanged**

```bash
cd target/container-tests/proof
niri msg -j workspaces | python3 -c 'import json,sys; [print(w["id"], w["idx"], w["output"], w.get("name"), w["is_focused"]) for w in sorted(json.load(sys.stdin), key=lambda w: w["id"])]' > workspaces.after
niri msg -j windows | python3 -c 'import json,sys; [print(w["id"], w["workspace_id"], w["app_id"]) for w in sorted(json.load(sys.stdin), key=lambda w: w["id"])]' > windows.after
task rc.hooks=0 rc.gc=off rc.json.array=on export 2>/dev/null | sha256sum > tasks.after
ls -l --time-style=full-iso "$(task _get rc.data.location)" > taskdata.after
readlink -f "$(command -v niritasks)" > niritasks.after; sha256sum "$(cat niritasks.after)" >> niritasks.after
systemctl --user show -p MainPID --value niri-tasks.service > service.after
for f in workspaces windows tasks taskdata niritasks service; do diff -u "$f.before" "$f.after" && echo "same: $f"; done
cd -
```

Expected: `same:` for all six. Any difference must be traced to the user's own activity during the run (ask them) — if it can be traced to the run, that is a bug in the wrapper; stop and report it.

And structurally — the container never had a host path to touch:

Run (while a run is going, from a second shell): `bash tests/container.sh bash -c 'sleep 30' >/dev/null 2>&1 & sleep 10; docker inspect $(docker ps -q --filter label=niritasks.tests.root="$PWD") --format '{{json .Mounts}} {{.HostConfig.NetworkMode}} {{.HostConfig.Privileged}}'; wait`
Expected: two mounts, both `"Type":"volume"` (`niritasks-tests-target-…` and `niritasks-tests-cargo`), no `"Type":"bind"`, network `bridge`, privileged `false`.

- [ ] **Step 4: Compare with the host's own results**

Run on the host, from this worktree (see memory: all.sh needs `NIRITASKS=` in a worktree):

```bash
cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh 2>&1 | tee target/container-tests/proof/host.log
```

Compare, suite by suite, the inner `passed: N   failed: M   skipped: K` lines and the final `PASS`/`FAIL`/`SKIP` list of `host.log` against `target/container-tests/latest.log`.

Expected: the same suites pass and none fails on either side. A skip on the host that the container does not have is fine and expected where the host has herdr on `NESTED_SPAWN_PATH` (`tests/e2e-panel.sh:734`); then the container's `passed` count for e2e-panel.sh is that much higher. Any other difference in counts is a finding: report it with both lines.

- [ ] **Step 5: From another worktree**

The main checkout gets these scripts only after the merge, so prove "from any worktree" with a scratch worktree of this branch:

```bash
git worktree add ../container-proof HEAD
(cd ../container-proof && bash tests/container.sh bash tests/e2e-tag.sh; echo "exit: $?")
git worktree remove ../container-proof
docker volume rm "niritasks-tests-target-$(printf '%s' "$(cd .. && pwd)/container-proof" | sha256sum | cut -c1-12)"
```

Expected: `exit: 0`, with no image rebuild (the Dockerfile is the same, so the tag is) and a fresh build directory volume for that worktree, removed afterwards.

- [ ] **Step 6: Commit the README time and any container fixes**

```bash
git add README.md tests/container
git commit -m "docs(readme): give tests/container.sh's measured run time

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If Step 2 needed container fixes, commit them first, separately, as `fix(container): …` with the cause in the body. `target/container-tests/proof/` stays uncommitted (it is under `target/`).

Report to the user: the container's and the host's summaries side by side, the six `same:` lines, the mount check, and the run time.
