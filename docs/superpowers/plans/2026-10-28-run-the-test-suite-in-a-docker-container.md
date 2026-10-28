# Run the Test Suite in a Docker Container Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `bash tests/container.sh` runs all five suites against the current checkout, uncommitted edits included, inside a Docker container, in the foreground or the background, and touches nothing on the machine: not its niri, workspaces, task database or installed `niritasks`.

**Architecture:** An image built from `ubuntu:26.04` (`tests/container/Dockerfile`) reproduces, piece by piece, what this machine gives the suites — every program they run, the same niri, Taskwarrior, compiler, fonts, GTK theme and font settings, locale, timezone and a uid-1000 user — and `tests/container/check-image.sh` proves each piece is there before anything is run. Inside a container, `tests/container/inside.sh` starts a headless sway, an outer niri as its one window with two named workspaces, builds the snapshot, and runs `tests/all.sh` unchanged. The wrapper `tests/container.sh` builds the image when its Dockerfile changes, streams a tar snapshot of the working tree in on stdin (no bind mounts), and tees everything to a log under `target/container-tests/`.

**Tech Stack:** Bash; Docker 29.1 (on the machine, user in the `docker` group); Ubuntu 26.04 with niri 26.04 from the `avengemedia/danklinux` PPA, sway 1.11 (wlroots headless + pixman), Mesa 26.0 (llvmpipe GL, lavapipe Vulkan), Rust 1.95.0 via rustup-init 1.29.2, Taskwarrior 3.5.0 compiled from its release tarball, Iosevka Term 34.8.1, Yaru GTK theme, dconf.

**Spec:** Taskwarrior task `f60e243e-83fa-45ef-9705-dc94cc3c0f0b` — its description and notes (`task rc.json.array=on f60e243e export`). No separate spec file.

## Global Constraints

- A Docker container, not a VM. A VM only if Task 2 or Task 4 proves niri cannot run in a container — then **stop and ask** the user before anything else.
- Image from `ubuntu:26.04`, everything baked in, so a run installs nothing (crates on a cold cargo cache are the one download).
- **Taskwarrior is 3.5.0, compiled from the release tarball, and nothing else.** Never apt's `taskwarrior` (2.6.2) — not directly, not as a dependency. The image build fails if `dpkg -s taskwarrior` succeeds or `task --version` is not exactly `3.5.0`.
- **VS Code is excluded** (user's decision, 2026-10-28): no `code`, no stub.
- Code goes in as a snapshot of the current working tree, uncommitted edits included, copied in at start; it builds there with `NIRITASKS` set to that build; the cargo cache lives in a Docker volume.
- Inside, a headless compositor runs an outer niri with two named workspaces, and `tests/all.sh` runs as it does now. `tests/all.sh`, `tests/e2e-*.sh`, `tests/lib/*` and `src/` are **not** changed to suit the container. If a check can only pass by changing one of them, stop and tell the user.
- Nothing on the host is touched: no bind mount of any host path, no host `XDG_RUNTIME_DIR`, Wayland, niri or D-Bus socket, no `--network host`, no `--privileged`. The only host writes are the log under the checkout's own `target/` and Docker's own image and volumes.
- The wrapper is `bash tests/container.sh`: builds the image if needed, copies the tree in, runs `tests/all.sh`, saves the log, and can run in the background.
- Done when: from any worktree the wrapper runs all five suites in the container with zero skips and the same results as on the host, the host's windows, workspaces and task database are unchanged, and README documents it beside `tests/all.sh`.
- Out of scope: CI, and replacing `tests/all.sh` or `try.sh` on the host.
- Commits: Conventional Commits, `test(container): …`, `docs(readme): …`, subject ≤72 characters, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` trailer.

## The inventory (checked on 2026-10-28, so you need not)

This is the heart of the plan: what the suites reach for on this machine, where it comes from here, and what the image does about it. Sources read: `install.sh`, every `Command::new` and `on_path` in `src/` and `tests/*.rs`, every program the three e2e scripts, `tests/lib/*` and `herdr/task-docs/tests/` run, the `-sys` crates in `Cargo.lock` and `ldd ~/.cargo/bin/niritasks`, and ubuntu-setup's `lib/manifest.sh`, `install.sh`, `configure.sh` and `home/`.

### In the image

| What | Used by | On this machine | In the image |
|---|---|---|---|
| niri 26.04 | e2e suites (outer + nested), `tests/all.sh` checks | `niri 26.04ppa3`, `ppa:avengemedia/danklinux` (ubuntu-setup adds it; **not** in Ubuntu's archive) | same PPA, same package |
| Taskwarrior 3.5.0 | everything | compiled from the release tarball, `cmake --install` | compiled from the same tarball (sha256 pinned), installed to `/usr/local` |
| rustc/cargo 1.95.0 | build, `cargo test` | rustup | rustup-init 1.29.2 (sha256 pinned), toolchain 1.95.0 minimal |
| GTK 4.22 + gtk4-layer-shell 1.3 dev, libwayland-dev, pkg-config, build-essential | the crate's `-sys` crates (gtk4, gdk4-wayland, gtk4-layer-shell, wayland-sys, pango, graphene, gio) | apt (ubuntu-setup `APT_BUILD_PACKAGES`) | apt |
| Iosevka Term 34.8.1 | panel and box font, by name (`src/panel/style.rs:12`); covers every UI glyph (▶ ● ○ × … — · ─ ✓ ✕, checked with `fc-match …:charset=`) | `~/.local/share/fonts/iosevka-term/`, 54 TTFs from the release zip (ubuntu-setup `IOSEVKA_VERSION`) | same zip (sha256 pinned), `/usr/local/share/fonts/iosevka-term` |
| Noto Sans, Noto Color Emoji | GTK's default font `Adwaita Sans 11` is not installed, so fontconfig falls back to `NotoSans-Regular.ttf`; emoji to `NotoColorEmoji.ttf` | `fonts-noto-core`, `fonts-noto-color-emoji` | same packages |
| GTK settings | every GTK window the e2e suites draw | via the portal from gsettings: theme `Yaru-purple-dark`, color-scheme `prefer-dark`, icon theme `Yaru-purple-dark`, cursor `Adwaita`, font `Adwaita Sans 11`, antialiasing `grayscale`, hinting `slight`, rgba `rgb`, text scaling `1.0`, monospace `Ubuntu Sans Mono 11` | the same keys in a dconf system db; `yaru-theme-gtk`, `yaru-theme-icon`, `adwaita-icon-theme` |
| GTK renderer | every GTK window | `VulkanRenderer` (NVIDIA) | `mesa-vulkan-drivers` (lavapipe), so GTK also picks Vulkan; Task 2 checks |
| Mesa GL/EGL | outer and nested niri (winit backend), alacritty | NVIDIA | `libgl1-mesa-dri`, `libegl-mesa0` (llvmpipe), `LIBGL_ALWAYS_SOFTWARE=1` |
| sway 1.11 | the headless compositor the outer niri runs in | — (your niri is on DRM) | apt; `WLR_BACKENDS=headless WLR_RENDERER=pixman` |
| wtype | e2e-box, e2e-panel keyboard checks | apt | apt |
| alacritty | e2e-box refocus check (skipped without it) | snap | apt 0.16.1 |
| python3, python3-pil | e2e frames, all JSON parsing in the scripts | apt | apt |
| python3-gi, gir1.2-gtk-4.0 | the GTK settings probe (Task 2), nothing else | apt | apt |
| fzf | project list ranking (`src/panel/matching.rs`); two cargo tests return early without it | apt (ubuntu-setup) | apt |
| git, util-linux (`flock`, `setsid`), coreutils (`timeout`, `realpath`), procps, tar, sqlite3 | e2e locks, plugin tests, `tests/backup.rs:143` | apt | apt |
| dbus-daemon / `dbus-run-session` | a session bus for GTK applications | your session bus | a private bus per run |
| libnotify-bin (`notify-send`), xdg-utils (`xdg-open`, `xdg-settings`) | `src/notify.rs`, `src/open.rs`; the e2e and plugin tests stub them on their own PATH | apt | apt, so un-stubbed code paths match the host's |
| locale `en_US.UTF-8` | Taskwarrior and Python text handling | `LANG=en_US.UTF-8` | `locales`, generated, `LANG` set |
| timezone | due dates (`due:friday` in e2e-box) | `Australia/Melbourne` | `tzdata`; the wrapper passes the host's zone as `TZ` |
| a uid-1000 user with `~/.taskrc` and `~/Projects/<folders>` | permission semantics; e2e-panel's project-list checks are skipped unless `~/Projects` holds a folder not named `e2e` (`tests/e2e-panel.sh:1008`, `:1056`) | `paul`, uid 1000; `~/.taskrc` has `data.location=$HOME/.task` | user `tester`, uid 1000; same `.taskrc` line; `~/Projects/alpha`, `~/Projects/beta` |
| session env | GTK, niri spawns | `XDG_CURRENT_DESKTOP=niri`, `XDG_SESSION_DESKTOP=niri`, `XDG_SESSION_TYPE=wayland` | set by `inside.sh` |

### Left out on purpose

| What | Why it is safe to leave out | What it changes |
|---|---|---|
| apt `taskwarrior` 2.6.2 (in ubuntu-setup's `APT_PACKAGES`) | the suites are written against 3.5.0 | — (the build refuses it) |
| VS Code (`code`) | user's decision | `programs::tests::the_editor_is_given_the_project_directory` returns early (passes without asserting), as on any machine without VS Code |
| herdr | every suite fakes it; `nested-niri.sh` strips it from PATH, and a herdr in `/usr/bin` would make e2e-panel and e2e-box **skip** their refine checks (`tests/e2e-panel.sh:734`, `tests/e2e-box.sh:401`) | `programs::tests::startup_commands_join_the_named_session_in_the_raw_folder` skips its one herdr-only assert |
| claude, wt (worktrunk), ghostty, gh, plannotator-tui | no suite runs them for real: they are faked (`wt`, `herdr`, `plannotator-tui`) or only an argv is asserted; the e2e spawn PATH (`$SB/bin:/usr/bin:/bin`) has none of them on the host either (they are in `~/.local/bin`), and gh would need your GitHub login | nothing measured |
| ffmpeg/ffplay, docker, nvidia-smi, curl's Kokoro use (`src/speak.rs`) | its unit tests only assert argv strings and run `cat` | nothing |
| mako, waybar, fuzzel, bubblewrap, socat, your niri config and window rules | desktop pieces no suite reaches: notifications are stubbed, the nested niri has its own config, the refine sandbox is never started | nothing |
| the installed `niritasks`, its systemd unit, `install.sh`'s links (skills, rules, refine mod, herdr plugin, niri include) | the suites run the snapshot's build through `NIRITASKS`, and the e2e scripts point `XDG_DATA_HOME` at a sandbox; `systemctl --user` missing makes `nested_service_untouched` compare `0` with `0` | nothing |

### Other facts

- In `ubuntu:26.04` with the PPA added, the image's whole apt list resolves (359 packages) and pulls in neither `taskwarrior` nor any missing package (checked with `apt-get install -s`).
- Spike, half done: in such an image `sway` (headless, pixman) runs an outer `niri` (`LIBGL_ALWAYS_SOFTWARE=1`) whose `niri msg workspaces` shows its two named workspaces, the first focused; a nested niri on it, started as `tests/lib/nested-niri.sh` does, answers `niri msg action screenshot-screen` with a PNG of the configured `#406080`. Harmless log noise: `EGL_EXT_device_drm` unsupported, no login1/locale1, no xcursor theme, no Xwayland. Sway's default headless output is 1920x1080, which leaves the outer niri 1916x1053.
- Taskwarrior 3.5.0: `https://github.com/GothenburgBitFactory/taskwarrior/releases/download/v3.5.0/task-3.5.0.tar.gz`, sha256 `9ea64b411f8314414f440ec765dfdf5a86c9f6159df47e2f60cff3db6b31157a`. CMake `3.24...4.0` policy range (apt's 4.2 is fine), C++17, libuuid, libsqlite3, Rust ≥ 1.91.1, network at configure. Must be **installed** (`cmake --install`), or it cannot find `default.theme` (upstream #4174).
- rustup-init 1.29.2: `https://static.rust-lang.org/rustup/archive/1.29.2/x86_64-unknown-linux-gnu/rustup-init`, sha256 `5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4`.
- Iosevka Term 34.8.1: `https://github.com/be5invis/Iosevka/releases/download/v34.8.1/PkgTTF-IosevkaTerm-34.8.1.zip`, sha256 `c9929ef8c27e5469906468e378e11d3023690ae9cd411a4af3de78190b901075`, the same 54 files.
- The worktree's `.git` is a file, so the snapshot is `git ls-files --cached --others --exclude-standard`; no suite needs the checkout to be a repository.
- `tests/all.sh` defaults `NIRITASKS` to the installed binary; on the host, in a worktree, run it as `NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh` after `cargo build`.

## File Structure

- Create `tests/container/Dockerfile` — the image: the inventory above, pinned, with build-time guards on Taskwarrior.
- Create `tests/container/dconf/00-machine` — the machine's GTK-relevant gsettings, as a dconf keyfile.
- Create `tests/container/check-image.sh` — proves the image holds the inventory; run against an image on stdin.
- Create `tests/container/gtk-settings.py` — prints what GTK sees (settings and renderer); run on the host and in the container, and diffed.
- Create `tests/container/sway.conf`, `tests/container/niri.kdl` — the headless compositor and the outer niri.
- Create `tests/container/inside.sh` — the in-container desktop, the build, then `tests/all.sh` or a given command.
- Create `tests/container.sh` — the wrapper on the host.
- Modify `README.md` — "Running the tests": the command, a table row, a subsection.

---

### Task 1: The image, and a check that proves it holds the inventory

Nothing is run in the image until this check passes: a missing piece shows up here as one line, not later as a failing pixel check.

**Files:**
- Create: `tests/container/check-image.sh`
- Create: `tests/container/dconf/00-machine`
- Create: `tests/container/Dockerfile`

**Interfaces:**
- Consumes: nothing.
- Produces: an image buildable with `docker build -t <tag> tests/container`, whose default user is `tester` (uid 1000, `HOME=/home/tester`), workdir `/work` (owned by `tester`, with `/work/target` existing and owned by `tester`), `CARGO_HOME=/opt/rust/cargo` (with `/opt/rust/cargo/registry` owned by `tester`), `RUSTUP_HOME=/opt/rust/rustup`, `LANG=en_US.UTF-8`. `bash -s < tests/container/check-image.sh` run in it prints one `ok`/`MISSING` line per item and exits with the number missing.

- [ ] **Step 1: Write the check**

`tests/container/check-image.sh`:

```bash
#!/usr/bin/env bash
# Whether the test image holds what the suites use on this machine: the
# inventory in docs/superpowers/plans/2026-10-28-run-the-test-suite-in-a-docker-container.md.
# Run inside the image, from the host:
#
#   docker run --rm -i <image> bash -s < tests/container/check-image.sh
#
# One line per item, ok or MISSING, and the exit status is how many are
# missing. A piece that is absent here would otherwise show up much later,
# as a suite skipped or a frame a few pixels off.
set -u

missing=0
ok()   { echo "  ok       $*"; }
miss() { echo "  MISSING  $*"; missing=$((missing+1)); }
check() {  # label, command...
    local label="$1"; shift
    if "$@" >/dev/null 2>&1; then ok "$label"; else miss "$label"; fi
}

echo "programs"
for p in niri sway alacritty wtype python3 git flock setsid timeout realpath \
         sqlite3 tar fzf xdg-open xdg-settings notify-send dbus-run-session \
         cargo rustc cc pkg-config task fc-match gsettings locale; do
    check "$p" command -v "$p"
done

echo "versions"
check "niri 26.04"          sh -c 'niri --version | grep -q "^niri 26\.04"'
check "task 3.5.0"          sh -c '[ "$(task --version)" = 3.5.0 ]'
check "rustc 1.95.0"        sh -c 'rustc --version | grep -q "^rustc 1\.95\.0 "'
check "no apt taskwarrior"  sh -c '! dpkg -s taskwarrior'

echo "left out on purpose"
for p in code herdr claude wt ghostty gh plannotator-tui; do
    check "no $p" sh -c "! command -v $p"
done

echo "build libraries"
for m in gtk4 gtk4-layer-shell-0 wayland-client; do
    check "pkg-config $m" pkg-config --exists "$m"
done

echo "python"
check "PIL"     python3 -c 'import PIL'
check "Gtk 4.0" python3 -c 'import gi; gi.require_version("Gtk", "4.0"); from gi.repository import Gtk'

echo "fonts"
check "Iosevka Term Extended" sh -c 'fc-match -f "%{file}" "Iosevka Term Extended" | grep -q "/IosevkaTerm-Extended.ttf$"'
check "54 Iosevka Term files" sh -c '[ "$(ls /usr/local/share/fonts/iosevka-term/*.ttf | wc -l)" = 54 ]'
check "sans is Noto Sans"     sh -c 'fc-match -f "%{file}" sans-serif | grep -q "/NotoSans-Regular.ttf$"'
check "emoji is Noto"         sh -c 'fc-match -f "%{file}" emoji | grep -q "/NotoColorEmoji.ttf$"'

echo "GTK settings"
want() {  # key, value as gsettings prints it
    check "$1 = $2" sh -c "[ \"\$(gsettings get org.gnome.desktop.interface $1)\" = \"$2\" ]"
}
want color-scheme        "'prefer-dark'"
want gtk-theme           "'Yaru-purple-dark'"
want icon-theme          "'Yaru-purple-dark'"
want cursor-theme        "'Adwaita'"
want font-name           "'Adwaita Sans 11'"
want monospace-font-name "'Ubuntu Sans Mono 11'"
want font-antialiasing   "'grayscale'"
want font-hinting        "'slight'"
want font-rgba-order     "'rgb'"
want text-scaling-factor "1.0"
check "Yaru-purple-dark gtk-4.0 theme" test -f /usr/share/themes/Yaru-purple-dark/gtk-4.0/gtk.css

echo "rendering"
check "lavapipe (Vulkan)" sh -c 'ls /usr/share/vulkan/icd.d/ | grep -q lvp'
check "llvmpipe (GL)"     sh -c 'ls /usr/lib/x86_64-linux-gnu/dri/ | grep -q swrast'

echo "user and session"
check "uid 1000"               sh -c '[ "$(id -u)" = 1000 ]'
check "HOME /home/tester"      sh -c '[ "$HOME" = /home/tester ]'
check "~/.taskrc"              grep -qx 'data.location=$HOME/.task' "$HOME/.taskrc"
check "~/Projects/alpha, beta" sh -c '[ -d "$HOME/Projects/alpha" ] && [ -d "$HOME/Projects/beta" ]'
check "LANG en_US.UTF-8"       sh -c '[ "$LANG" = en_US.UTF-8 ] && locale -a | grep -qix en_US.utf8'
check "tzdata"                 test -f /usr/share/zoneinfo/Australia/Melbourne
check "/work writable"         test -w /work
check "/work/target writable"  test -w /work/target
check "cargo registry writable" test -w "${CARGO_HOME:-/nonexistent}/registry"

echo
echo "$missing missing"
exit "$missing"
```

- [ ] **Step 2: Run it against the bare base image, to see it fail**

Run: `docker run --rm -i ubuntu:26.04 bash -s < tests/container/check-image.sh; echo "exit: $?"`
Expected: mostly `MISSING` lines, ending `N missing` with N > 40, and a non-zero exit. (Only things like `tar`, `timeout`, `realpath`, `setsid`, `flock` and the `no …` lines are `ok`.)

- [ ] **Step 3: Write the dconf keyfile**

`tests/container/dconf/00-machine` — the values are this machine's `gsettings get org.gnome.desktop.interface …` on 2026-10-28; GTK reads them through the portal there and straight from dconf here:

```
# The machine's GTK look, which every GTK window the e2e suites draw is
# drawn with. Copied from `gsettings get org.gnome.desktop.interface <key>`
# on the machine; tests/container/gtk-settings.py, run on both sides, is
# how a drift shows.
[org/gnome/desktop/interface]
color-scheme='prefer-dark'
gtk-theme='Yaru-purple-dark'
icon-theme='Yaru-purple-dark'
cursor-theme='Adwaita'
font-name='Adwaita Sans 11'
monospace-font-name='Ubuntu Sans Mono 11'
font-antialiasing='grayscale'
font-hinting='slight'
font-rgba-order='rgb'
text-scaling-factor=1.0
```

- [ ] **Step 4: Write the Dockerfile**

`tests/container/Dockerfile`:

```dockerfile
# The image tests/container.sh runs the test suites in. It reproduces what
# the suites reach for on the machine — programs, versions, fonts, GTK look,
# locale, user — and tests/container/check-image.sh proves it does. The
# inventory, with where each piece comes from and what is left out on
# purpose, is in
# docs/superpowers/plans/2026-10-28-run-the-test-suite-in-a-docker-container.md.
# tests/container.sh tags the image with this file's hash, so any edit here
# builds a new image on the next run.

FROM ubuntu:26.04 AS base
ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rust/rustup CARGO_HOME=/opt/rust/cargo \
    PATH=/opt/rust/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl git build-essential pkg-config \
 && rm -rf /var/lib/apt/lists/*
# The machine's compiler, so a build here is the build there.
RUN curl -fsSLo /tmp/rustup-init https://static.rust-lang.org/rustup/archive/1.29.2/x86_64-unknown-linux-gnu/rustup-init \
 && echo "5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.95.0 \
 && rm /tmp/rustup-init

# Taskwarrior 3.5.0, compiled from the release tarball as on the machine.
# Never apt's taskwarrior, which is 2.6.2. Installed, not run from the build
# tree: 3.5.0 cannot find default.theme otherwise (upstream #4174).
FROM base AS taskwarrior
RUN apt-get update \
 && apt-get install -y --no-install-recommends cmake uuid-dev libsqlite3-dev \
 && rm -rf /var/lib/apt/lists/*
RUN curl -fsSLo /tmp/task.tar.gz https://github.com/GothenburgBitFactory/taskwarrior/releases/download/v3.5.0/task-3.5.0.tar.gz \
 && echo "9ea64b411f8314414f440ec765dfdf5a86c9f6159df47e2f60cff3db6b31157a  /tmp/task.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/task.tar.gz -C /tmp \
 && cmake -S /tmp/task-3.5.0 -B /tmp/task-build -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr/local \
 && cmake --build /tmp/task-build -j"$(nproc)" \
 && cmake --install /tmp/task-build

FROM base
ENV LANG=en_US.UTF-8
# niri 26.04 is not in Ubuntu's archive: the machine's comes from the
# avengemedia/danklinux PPA, and so does this one. sway is only the headless
# compositor the outer niri runs in; Mesa is the software GL and Vulkan.
# Everything else is in the inventory.
RUN apt-get update \
 && apt-get install -y --no-install-recommends software-properties-common gpg-agent \
 && add-apt-repository -y ppa:avengemedia/danklinux \
 && apt-get install -y --no-install-recommends \
      niri sway libgl1-mesa-dri libegl-mesa0 libegl1 mesa-vulkan-drivers libvulkan1 \
      dbus-daemon dbus-bin \
      libgtk-4-dev libgtk4-layer-shell-dev libwayland-dev libuuid1 libsqlite3-0 \
      wtype alacritty python3 python3-pil python3-gi gir1.2-gtk-4.0 \
      util-linux procps sqlite3 unzip fzf xdg-utils libnotify-bin \
      fontconfig fonts-noto-core fonts-noto-color-emoji \
      adwaita-icon-theme yaru-theme-gtk yaru-theme-icon hicolor-icon-theme glycin-loaders librsvg2-common \
      dconf-gsettings-backend dconf-cli gsettings-desktop-schemas \
      locales tzdata \
 && rm -rf /var/lib/apt/lists/* \
 && locale-gen en_US.UTF-8
COPY --from=taskwarrior /usr/local/bin/task /usr/local/bin/task
COPY --from=taskwarrior /usr/local/share/doc/task /usr/local/share/doc/task
RUN ! dpkg -s taskwarrior >/dev/null 2>&1 && [ "$(task --version)" = 3.5.0 ]

# The panel and the box draw in Iosevka Term Extended, by name
# (src/panel/style.rs). The machine's version, from the same release zip.
RUN curl -fsSLo /tmp/iosevka.zip https://github.com/be5invis/Iosevka/releases/download/v34.8.1/PkgTTF-IosevkaTerm-34.8.1.zip \
 && echo "c9929ef8c27e5469906468e378e11d3023690ae9cd411a4af3de78190b901075  /tmp/iosevka.zip" | sha256sum -c - \
 && mkdir -p /usr/local/share/fonts/iosevka-term \
 && unzip -q -j /tmp/iosevka.zip '*.ttf' -d /usr/local/share/fonts/iosevka-term \
 && rm /tmp/iosevka.zip \
 && fc-cache -f

# The machine's GTK look, as a dconf system database GSettings reads with no
# dconf service running.
COPY dconf/00-machine /etc/dconf/db/local.d/00-machine
RUN printf 'user-db:user\nsystem-db:local\n' > /etc/dconf/profile/user && dconf update

# A uid-1000 user, as on the machine, with what its home holds there: a
# .taskrc, and ~/Projects with folders in it, without which e2e-panel.sh
# skips its project-list checks. /work and the cargo registry are made here,
# owned by it, so the Docker volumes mounted on them start out owned by it.
# The base image's own uid-1000 user, ubuntu, makes way for it.
RUN userdel -r ubuntu \
 && useradd -m -u 1000 -s /bin/bash tester \
 && printf 'data.location=$HOME/.task\n' > /home/tester/.taskrc \
 && mkdir -p /home/tester/Projects/alpha /home/tester/Projects/beta /work/target /opt/rust/cargo/registry \
 && chown -R tester:tester /home/tester /work /opt/rust
USER tester
WORKDIR /work
```

Notes for the implementer:
- If `unzip -j … '*.ttf'` finds the files under a subfolder, `-j` already flattens it; the check wants exactly 54 TTFs.

- [ ] **Step 5: Build the image and run the check**

```bash
docker build -t niritasks-tests:dev tests/container
docker run --rm -i niritasks-tests:dev bash -s < tests/container/check-image.sh; echo "exit: $?"
```

The first build takes several minutes: Taskwarrior compiles, and the apt list is about 360 packages. Expected: every line `ok`, `0 missing`, `exit: 0`.

A `MISSING` line is fixed in the Dockerfile (or the keyfile), then rebuild and rerun — never by loosening the check. If a value can't be matched (say `gsettings get` prints a font name differently), compare against `gsettings get org.gnome.desktop.interface <key>` on the host, and stop and ask if the host's own value is now different from the plan's.

- [ ] **Step 6: Commit**

```bash
chmod +x tests/container/check-image.sh
git add tests/container
git commit -m "test(container): add the test image and a check of what it holds

A ubuntu:26.04 image with what the suites use on the machine: niri 26.04
from the same PPA, Taskwarrior 3.5.0 compiled from its tarball (never
apt's 2.6), rustc 1.95.0, Iosevka Term 34.8.1, the machine's GTK look
as a dconf database, en_US.UTF-8 and a uid-1000 user. check-image.sh
lists each piece as ok or missing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Keep `niritasks-tests:dev` for Tasks 2 and 4.

---

### Task 2: The outer niri, and the GTK look matching the machine's

The first half of the task's spike gate: niri runs in the container, and a GTK window there is drawn with the machine's settings and renderer. Still no suite runs.

**Files:**
- Create: `tests/container/sway.conf`
- Create: `tests/container/niri.kdl`
- Create: `tests/container/inside.sh`
- Create: `tests/container/gtk-settings.py`

**Interfaces:**
- Consumes: the image from Task 1 (`niritasks-tests:dev`), user `tester`, workdir `/work`.
- Produces: `bash tests/container/inside.sh [--no-build] [command...]`, run from `/work` (the snapshot) under `dbus-run-session`: stands up sway and the outer niri, exports `NIRI_SOCKET`, `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, then builds with `cargo build --locked` and exports `NIRITASKS=/work/target/debug/niritasks` (skipped with `--no-build`), then runs the command (default `bash tests/all.sh`) and exits with its status; exits 1 with `container: …` on stderr when the desktop or the build cannot start. `python3 tests/container/gtk-settings.py` prints `name value` lines, the last `renderer <GskRenderer class>`.

- [ ] **Step 1: Write the probe, and record the host's answer**

`tests/container/gtk-settings.py`:

```python
"""What GTK sees: the settings that decide how a window is drawn, and the
renderer it draws with. Run on the machine and in the test container
(tests/container/inside.sh) and diff the two; they should be the same.

    python3 tests/container/gtk-settings.py
"""
import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk  # noqa: E402

SETTINGS = [
    "gtk-theme-name", "gtk-interface-color-scheme", "gtk-application-prefer-dark-theme",
    "gtk-font-name", "gtk-xft-antialias", "gtk-xft-hinting", "gtk-xft-hintstyle",
    "gtk-xft-rgba", "gtk-xft-dpi", "gtk-hint-font-metrics", "gtk-font-rendering",
    "gtk-icon-theme-name", "gtk-cursor-theme-name", "gtk-enable-animations",
]


def main():
    settings = Gtk.Settings.get_default()
    for name in SETTINGS:
        print(name, settings.get_property(name))
    window = Gtk.Window()
    window.present()
    loop = GLib.MainLoop()

    def report():
        print("renderer", type(window.get_renderer()).__name__)
        loop.quit()
        return False

    GLib.timeout_add(500, report)
    loop.run()


main()
```

Run on the host: `python3 tests/container/gtk-settings.py | tee target/gtk-host.txt`
Expected (2026-10-28):

```
gtk-theme-name Yaru-purple-dark
gtk-interface-color-scheme 2
gtk-application-prefer-dark-theme False
gtk-font-name Adwaita Sans 11
gtk-xft-antialias 1
gtk-xft-hinting 1
gtk-xft-hintstyle hintslight
gtk-xft-rgba none
gtk-xft-dpi 98304
gtk-hint-font-metrics True
gtk-font-rendering 0
gtk-icon-theme-name Yaru-purple-dark
gtk-cursor-theme-name Adwaita
gtk-enable-animations True
renderer VulkanRenderer
```

(A window flashes on your workspace for half a second.)

- [ ] **Step 2: Write the compositor and outer niri configs**

`tests/container/sway.conf`:

```
# The headless compositor the container's outer niri runs in
# (tests/container/inside.sh). niri needs one: its own headless backend is
# only for niri's tests. One output big enough that the 1600x1000 nested
# niri the e2e tests park fits on the outer niri with room to spare.
output * resolution 2560x1440
default_border none
gaps inner 0
xwayland disable
```

`tests/container/niri.kdl`:

```kdl
// The outer niri in the test container (tests/container/inside.sh): the
// niri tests/all.sh takes for yours. Two named workspaces, the first
// focused, as tests/e2e-tag.sh wants, named like the ~/Projects folders the
// image makes; the e2e tests park their nested niri on the empty workspace
// niri keeps after them. The startup line is how inside.sh learns this
// niri's sockets.
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
#   bash tests/container/inside.sh --no-build <command>     # the desktop only
#
# It stands up the desktop tests/all.sh expects to find on the machine, then
# gets out of the way: a headless sway, an outer niri as its one window with
# two named workspaces, and the session variables niri's own session sets.
# tests/all.sh then runs as it does on the desktop, and its e2e suites nest
# their own niri inside this one. Nothing here is the machine's: no socket,
# file or display of the host reaches the container.
#
# It builds the snapshot first and points NIRITASKS at that build: there is
# no installed niritasks in here to test by mistake.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

die() { echo "container: $*" >&2; exit 1; }

build=1
[ "${1:-}" = --no-build ] && { build=""; shift; }

export XDG_RUNTIME_DIR="/tmp/runtime-$(id -u)"
mkdir -p -m 700 "$XDG_RUNTIME_DIR"
# What niri's session sets on the machine.
export XDG_CURRENT_DESKTOP=niri XDG_SESSION_DESKTOP=niri XDG_SESSION_TYPE=wayland
# Mesa's llvmpipe for every GL client — the outer niri, the nested niris,
# alacritty. GTK draws with Vulkan, as on the machine, on Mesa's lavapipe.
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

if [ -n "$build" ]; then
    echo "building $(pwd)"
    cargo build --locked || die "cargo build failed"
    export NIRITASKS="$PWD/target/debug/niritasks"
fi

[ $# -gt 0 ] || set -- bash tests/all.sh
"$@"
```

- [ ] **Step 4: Run the outer niri and the probe in the container, and diff**

A one-off run, the way the wrapper (Task 3) will do it, minus the volumes:

```bash
chmod +x tests/container/inside.sh
git ls-files -z --cached --others --exclude-standard |
    while IFS= read -r -d '' f; do { [ -e "$f" ] || [ -L "$f" ]; } && printf '%s\0' "$f"; done |
    tar --null -T - -cf - |
    docker run --rm -i --init -e TZ="$(cat /etc/timezone)" niritasks-tests:dev \
        bash -c 'tar -xf - -C /work && exec dbus-run-session -- bash /work/tests/container/inside.sh --no-build sh -c "niri msg outputs; python3 tests/container/gtk-settings.py"' \
    | tee target/gtk-container.txt
diff <(grep -E '^(gtk-|renderer )' target/gtk-host.txt) <(grep -E '^(gtk-|renderer )' target/gtk-container.txt) && echo "GTK: same"
```

Expected: `niri msg workspaces` listing `alpha` (focused) and `beta` and an empty third; `niri msg outputs` with a logical size of at least 1600x1000 (about 2556x1413); and `GTK: same`.

If they differ, one line at a time:
- `gtk-theme-name`, `gtk-interface-color-scheme`, `gtk-font-name`, `gtk-xft-*`, `gtk-icon-theme-name`, `gtk-cursor-theme-name`: GTK is not reading the dconf database. Check `gsettings get org.gnome.desktop.interface gtk-theme` in the container prints `'Yaru-purple-dark'` (Task 1's check did) and `GSETTINGS_BACKEND` is unset; GTK on Wayland falls back to GSettings when there is no portal. If it still differs, stop and ask.
- `gtk-xft-dpi`: the outer output's scale is not 1. Add `output "winit" { scale 1; }` to `tests/container/niri.kdl`.
- `gtk-enable-animations`: the same on both sides is all that matters; it is a GTK default.
- `renderer GLRenderer` or `CairoRenderer` instead of `VulkanRenderer`: lavapipe is not usable over Wayland here. Run with `GSK_DEBUG=renderer` to see why; if lavapipe cannot be made to work, accept `GLRenderer` (llvmpipe), say so in a comment in `inside.sh`, and tell the user the renderer differs.
- The outer output smaller than 1600x1000: `output * resolution` did not apply to sway's headless output; use `output HEADLESS-1 mode --custom 2560x1440`.

**Stop and ask the user** if the outer niri does not start at all: that is the task's "niri cannot run in a container" gate.

- [ ] **Step 5: Commit**

```bash
git add tests/container
git commit -m "test(container): stand up an outer niri with the machine's GTK look

inside.sh starts a headless sway and an outer niri with two named
workspaces, builds the snapshot, and runs tests/all.sh or a given
command. gtk-settings.py prints what GTK sees; it gives the same answer
on the machine and in the container.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The wrapper, tests/container.sh

**Files:**
- Create: `tests/container.sh`

**Interfaces:**
- Consumes: `tests/container/Dockerfile` (Task 1); `bash /work/tests/container/inside.sh [command...]` (Task 2), run under `dbus-run-session` as `tester` with `/work/target` and `/opt/rust/cargo/registry` mounted.
- Produces: `bash tests/container.sh [command...]` — exits with the in-container command's status (default `bash tests/all.sh`); 1 when Docker is missing or unreachable, the image cannot be built, or a run of the same checkout is already going; 130 when interrupted. Writes the whole run to `target/container-tests/<UTC stamp>-<pid>.log`, `latest.log` pointing at it. Containers carry `niritasks.tests.root=<checkout path>`; volumes `niritasks.tests=1`. With `NIRITASKS_CONTAINER_KEEP=1` the container is kept after the run and its name printed, for `docker cp`.

- [ ] **Step 1: Run the command that should not exist yet**

Run: `bash tests/container.sh; echo "exit: $?"`
Expected: `No such file or directory`, `exit: 127`.

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
# This runs the same tests/all.sh, unchanged, on a desktop of the
# container's own (tests/container/inside.sh) in an image that reproduces
# what the suites use on the machine (tests/container/Dockerfile). Your
# niri, workspaces, task database and niritasks are never in reach: the
# container gets no mount, socket or display of the machine's, only the
# files of this checkout, copied in on stdin.
#
# The copy is what git would see of the working tree: tracked files as they
# are now, edited or not, and untracked ones that are not ignored. Not
# target/, not .git.
#
# The image is built the first time, and again only when the Dockerfile
# changes: it is tagged with the file's hash. The cargo registry lives in
# one Docker volume shared by every checkout, and each checkout's build
# directory in a volume of its own, so a second run rebuilds only what
# changed. Two runs of the same checkout at once would share that build
# directory, so the second refuses; runs of different worktrees go side by
# side.
#
# Everything the run prints is also saved, in target/container-tests/, with
# latest.log the newest. It exits as tests/all.sh does: non-zero only when a
# suite that ran said no. NIRITASKS_CONTAINER_KEEP=1 keeps the container
# afterwards, to copy a failed run's files out of it with docker cp.
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
# The machine's timezone, so a due date means the same day in here.
TZ_NAME=$(cat /etc/timezone 2>/dev/null || timedatectl show -p Timezone --value 2>/dev/null || echo UTC)

if [ -n "$(docker ps -aq --filter "label=niritasks.tests.root=$ROOT" --filter status=running)" ]; then
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

keep=()
if [ -n "${NIRITASKS_CONTAINER_KEEP:-}" ]; then
    echo "keeping the container afterwards: $NAME"
else
    keep=(--rm)
    # Interrupted, the container goes too, rather than run on unseen.
    trap 'docker rm -f "$NAME" >/dev/null 2>&1' EXIT
fi
trap 'exit 130' INT TERM

# Waited for in the background: bash runs a trap only once the foreground
# command is over, and docker run is not over until the suites are. $! is
# the pipeline's last command, docker run, whose status is the run's.
snapshot | docker run "${keep[@]}" -i --init --name "$NAME" \
    --label "niritasks.tests.root=$ROOT" \
    --shm-size 512m -e TZ="$TZ_NAME" \
    -v "$TARGET_VOLUME:/work/target" \
    -v "$REGISTRY_VOLUME:/opt/rust/cargo/registry" \
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
- `"$@"` after `inside` passes the optional command through `bash -c` intact (`inside` becomes `$0`).
- The user is the image's (`tester`); the volumes start out owned by it because their mount points are (Task 1).

- [ ] **Step 3: Run the desktop alone through it**

Run: `chmod +x tests/container.sh && bash tests/container.sh --no-build niri msg workspaces; echo "exit: $?"`
Expected: `repo` and `log` lines, the image build if the Dockerfile has no image yet, then `niri msg workspaces` twice (once from `inside.sh`'s own check) showing `alpha` and `beta`, `exit: 0`.

Run: `ls -l target/container-tests/ && tail -n 2 target/container-tests/latest.log`
Expected: one `<stamp>-<pid>.log`, `latest.log -> <that file>`, ending with the `log` line.

- [ ] **Step 4: Check the build, the cache and the exit status**

Run: `time bash tests/container.sh true; echo "exit: $?"` twice.
Expected: the first prints cargo fetching and compiling (minutes); the second prints `Finished` with nothing compiled, in well under a minute; both `exit: 0`.

Run: `bash tests/container.sh false; echo "exit: $?"`
Expected: `exit: 1`.

- [ ] **Step 5: Check the refusals and the interrupt**

```bash
bash tests/container.sh --no-build sleep 60 >/dev/null 2>&1 &
sleep 15; bash tests/container.sh --no-build true; echo "exit: $?"; wait
```

Expected: `a container run of /home/…/task-test-run-the-test-suite-in-a-docker-f60e243e is already going, and two would share one build directory`, `exit: 1`.

```bash
bash tests/container.sh --no-build sleep 300 >/dev/null 2>&1 & pid=$!
sleep 20; kill -TERM "$pid"; wait "$pid"; echo "exit: $?"
docker ps -aq --filter label=niritasks.tests.root="$PWD" | wc -l
```

Expected: `exit: 130`, then `0` — the container went at once, not after 300s. (TERM, not INT: a background job of a non-interactive shell ignores INT; Ctrl-C on a foreground run reaches the same trap.)

- [ ] **Step 6: Commit**

```bash
git add tests/container.sh
git commit -m "test(container): add tests/container.sh to run the suites in Docker

Builds the image when its Dockerfile changes, copies the working tree in
on stdin, runs tests/all.sh there and keeps the log in
target/container-tests/. No mount or socket of the machine reaches the
container.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Every suite in the container, e2e-panel.sh first

The second half of the spike gate, then the rest. Each suite is run alone first, so a failure has one place to come from.

**Files:**
- Modify only if a suite fails or skips: `tests/container/Dockerfile`, `tests/container/dconf/00-machine`, `tests/container/inside.sh`, `tests/container/niri.kdl`, `tests/container/sway.conf` (and `tests/container/check-image.sh` with any inventory change)

**Interfaces:**
- Consumes: `bash tests/container.sh [command...]` (Task 3), `NIRITASKS_CONTAINER_KEEP=1`.
- Produces: `target/container-tests/host.log` — the host's own `tests/all.sh` run, for Task 5.

- [ ] **Step 1: Record the host's results**

Tell the user first: this runs the e2e suites on their desktop, parking a nested niri on a spare workspace; keep off it for about two minutes.

```bash
cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh 2>&1 | tee target/container-tests/host.log
grep -E '^(passed:|  (PASS|SKIP|FAIL) |[0-9]+ passed)' target/container-tests/host.log
```

Expected: five suites `PASS`. Note each suite's inner `passed: N   failed: M   skipped: K` line and `cargo test`'s `test result:` lines — they are what the container must match.

- [ ] **Step 2: e2e-panel.sh — the gate**

Run: `bash tests/container.sh bash tests/e2e-panel.sh; echo "exit: $?"`
Expected: `passed: N   failed: 0   skipped: 0` with N equal to the host's e2e-panel `passed`, and `exit: 0`.

If it fails, keep the frames and logs:

```bash
NIRITASKS_CONTAINER_KEEP=1 bash tests/container.sh env NIRITASKS_E2E_KEEP=1 bash tests/e2e-panel.sh
# the run prints "keeping the container afterwards: <name>" and "the run's files are kept in /tmp/tmp.XXXX"
mkdir -p target/container-tests/debug
docker cp <name>:/tmp/tmp.XXXX target/container-tests/debug/run
docker cp <name>:/tmp/niri.log target/container-tests/debug/outer-niri.log
docker rm <name>
```

Look at the frames (`target/container-tests/debug/run/shots/*.png`) with the Read tool, beside the same frame from a host run kept the same way (`NIRITASKS_E2E_KEEP=1 NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`). Since Tasks 1–2 already pinned fonts, theme and renderer, what is left to differ is mostly speed and size:
1. `never held still`, `ten frames, no two alike`, a slide that does not finish, or `the daemon exited at once`: software rendering is slower. Check the container sees every CPU (`bash tests/container.sh --no-build nproc` against `nproc`) and give Docker more if not. Do not lengthen a test's waits; tell the user.
2. An edge a few pixels off: compare the two frames; the Task 2 probe already says the GTK side is the same, so look at the niri side (`niri msg outputs` scale inside the nested niri: `output "winit" { scale 1; }` is in its config).
3. Anything else: stop and ask, with both frames.

**Stop and ask the user** if e2e-panel.sh cannot pass in the container without changing a test or `src/`.

- [ ] **Step 3: The other four, one at a time**

```bash
bash tests/container.sh cargo test; echo "exit: $?"
bash tests/container.sh python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs; echo "exit: $?"
bash tests/container.sh bash tests/e2e-tag.sh; echo "exit: $?"
bash tests/container.sh bash tests/e2e-box.sh; echo "exit: $?"
```

Expected: each `exit: 0`, with the same counts as the host (`test result: ok. N passed` per cargo test binary; `Ran N tests … OK`; e2e-tag and e2e-box `passed: N   failed: 0`, e2e-box `skipped: 0`). Fix a failure on the container side as in Step 2, and add anything new to the inventory in `check-image.sh` too, so the check keeps describing the image.

- [ ] **Step 4: Commit any fixes**

If Steps 2–3 changed anything: `git add tests/container && git commit` with `fix(container): …` and the cause in the body (e.g. "alacritty needs …"). If nothing changed, there is nothing to commit.

---

### Task 5: README, and proof of the done-when

**Files:**
- Modify: `README.md` — "Running the tests" (code block around line 595, table around line 614, a subsection before the `> [!IMPORTANT]` box around line 675)

**Interfaces:**
- Consumes: `bash tests/container.sh` (Task 3), `target/container-tests/host.log` (Task 4).
- Produces: the README section; the evidence in the final report.

- [ ] **Step 1: Record the host's state before**

```bash
mkdir -p target/container-tests/proof && cd target/container-tests/proof
niri msg -j workspaces | python3 -c 'import json,sys; [print(w["id"], w["idx"], w["output"], w.get("name"), w["is_focused"]) for w in sorted(json.load(sys.stdin), key=lambda w: w["id"])]' > workspaces.before
niri msg -j windows | python3 -c 'import json,sys; [print(w["id"], w["workspace_id"], w["app_id"]) for w in sorted(json.load(sys.stdin), key=lambda w: w["id"])]' > windows.before
task rc.hooks=0 rc.gc=off rc.json.array=on export 2>/dev/null | sha256sum > tasks.before
ls -l --time-style=full-iso "$(task _get rc.data.location)" > taskdata.before
readlink -f "$(command -v niritasks)" > niritasks.before; sha256sum "$(cat niritasks.before)" >> niritasks.before
systemctl --user show -p MainPID --value niri-tasks.service > service.before
cd -
```

Tell the user the whole suite is about to run in the container, and ask them not to add, edit or complete tasks, or open or close windows, until it finishes — otherwise before and after prove nothing.

- [ ] **Step 2: Run everything, and check what the container could reach**

Run: `time bash tests/container.sh; echo "exit: $?"`
Expected: `5 passed   0 failed   0 skipped`, every inner summary `skipped: 0`, `exit: 0`. Note the wall time.

While it runs, from a second shell: `docker inspect $(docker ps -q --filter label=niritasks.tests.root="$PWD") --format '{{json .Mounts}} {{.HostConfig.NetworkMode}} {{.HostConfig.Privileged}}'`
Expected: two mounts, both `"Type":"volume"`, no `"Type":"bind"`; `bridge`; `false`.

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

Expected: six `same:` lines. A difference must be traced to the user's own activity (ask them); one traced to the run is a wrapper bug: stop and report it.

- [ ] **Step 4: Compare with the host's results**

Put `target/container-tests/host.log` (Task 4 Step 1) beside `target/container-tests/latest.log` and compare, suite by suite, the inner `passed/failed/skipped` lines and cargo test's `test result:` lines.

Expected: identical counts. The two early-returning cargo tests (VS Code's, herdr's one assert — see the inventory) pass on both sides, so they do not change the counts. Any other difference is a finding: report it with both lines.

- [ ] **Step 5: From another worktree**

The main checkout gets these scripts only after the merge, so prove "from any worktree" with a scratch worktree of this branch:

```bash
git worktree add ../container-proof HEAD
(cd ../container-proof && bash tests/container.sh bash tests/e2e-tag.sh; echo "exit: $?")
git worktree remove ../container-proof
docker volume rm "niritasks-tests-target-$(printf '%s' "$(cd .. && pwd)/container-proof" | sha256sum | cut -c1-12)"
```

Expected: `exit: 0`, no image rebuild (same Dockerfile, same tag), and a build directory volume of its own, removed afterwards.

- [ ] **Step 6: Document it in README.md**

In "### Running the tests", add under `tests/all.sh` in the code block, with the warm time from Step 2 rounded like its neighbours:

```bash
bash tests/all.sh           # everything this machine can run       ~100s
bash tests/container.sh     # everything, in a Docker container     ~NNNs
```

In the Needs/Touches table, directly under the `tests/all.sh` row:

```markdown
| `tests/container.sh` | Docker, and the user in the `docker` group. The first run builds the image (several minutes) and needs the network | Nothing of yours. Docker's own image and volumes (`niritasks-tests*`), and its log in `target/container-tests/` |
```

After the paragraph ending "…and a long note list scrolls, keeping a row made by Enter at the bottom in view." and before the `> [!IMPORTANT]` box:

````markdown
### Running the tests in a container

`tests/container.sh` runs `tests/all.sh` in a Docker container, against this
checkout as it stands, uncommitted edits included. Nothing of your desktop is
in reach: the container gets no mount, socket or display of yours, only a copy
of the files git would see, so your niri, workspaces, task database and
installed `niritasks` are never touched, and there is no workspace to keep off.
Run it from any worktree, beside other worktrees' runs, in the foreground or
in the background:

```bash
bash tests/container.sh                          # tests/all.sh
bash tests/container.sh bash tests/e2e-box.sh    # one suite
bash tests/container.sh >/dev/null 2>&1 &        # in the background
tail -f target/container-tests/latest.log
```

The image (`tests/container/Dockerfile`) reproduces what the suites use here:
niri 26.04 from the same PPA, Taskwarrior 3.5.0 compiled from its tarball,
rustc 1.95.0, Iosevka Term, the GTK theme and font settings, `en_US.UTF-8`,
your timezone and a uid-1000 user. VS Code, herdr, Claude Code, worktrunk,
ghostty and gh are left out: the suites fake or skip past them.
`docker run --rm -i <image> bash -s < tests/container/check-image.sh` lists
what it holds. Inside, `tests/container/inside.sh` starts a headless sway, an
outer niri in it with two named workspaces, builds the copy and runs
`tests/all.sh` with `NIRITASKS` set to that build; the e2e suites nest their
niri in the outer one as they would in yours.

The image is built on the first run and again only when the Dockerfile
changes; cargo's registry and each checkout's build directory live in Docker
volumes, so later runs rebuild only what changed. A second run of the same
checkout while one is going refuses, since the two would share a build
directory. Every run is logged in `target/container-tests/`, `latest.log` the
newest, and it exits as `tests/all.sh` does. `NIRITASKS_CONTAINER_KEEP=1`
keeps the container for `docker cp` after a failure. To reclaim the space:

```bash
docker volume rm $(docker volume ls -q --filter label=niritasks.tests=1)
docker image rm $(docker image ls -q niritasks-tests)
```
````

(The outer four-backtick fence is this plan's; in the README the section is plain Markdown with two ```` ```bash ```` blocks.)

- [ ] **Step 7: Commit and report**

```bash
docker image rm niritasks-tests:dev
git add README.md
git commit -m "docs(readme): document running the tests in a container

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

`target/container-tests/` stays uncommitted (it is under `target/`). Report to the user: the host's and the container's summaries side by side, the six `same:` lines, the mount check, the GTK probe diff from Task 2, and the run time.
