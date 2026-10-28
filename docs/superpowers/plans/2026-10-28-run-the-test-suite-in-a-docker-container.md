# Run the Test Suite in a Docker Container Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `bash tests/container.sh` runs all five suites against the current checkout, uncommitted edits included, inside a Docker container, in the foreground or the background, and touches nothing on the machine: not its niri, workspaces, task database or installed `niritasks`.

**Architecture:** An image built from `ubuntu:26.04` (`tests/container/Dockerfile`) reproduces what this machine gives the suites — every program they run, at the machine's versions and in the machine's folders, the same niri, Taskwarrior, compiler, fonts, GTK look, locale, timezone and a uid-1000 user — plus this crate's dependencies already compiled. `tests/container/check-image.sh` proves the image is as designed; `tests/container/parity.sh`, run on both sides at every run, warns when the machine has moved on from the image. Inside a container with no network and no host mounts, `tests/container/inside.sh` starts a headless sway, an outer niri set up like the machine's, makes the links `install.sh` makes, builds the snapshot and runs `tests/all.sh` unchanged. The wrapper `tests/container.sh` gets the image from `tests/container/image.sh` (named after its inputs, built when they change), streams a tar snapshot of the working tree in on stdin, keeps each checkout's build directory in a Docker volume seeded from the image, tees everything to a log, and copies a failed run's logs and frames out.

**Tech Stack:** Bash; Docker 29.1 (on the machine, user in the `docker` group, BuildKit); Ubuntu 26.04 (digest-pinned) with niri 26.04 and ghostty from the `avengemedia/danklinux` PPA, sway 1.11 (wlroots headless + pixman), Mesa 26.0 (llvmpipe GL, lavapipe Vulkan), Rust 1.95.0 via rustup-init 1.29.2, Taskwarrior 3.5.0 compiled from its release tarball, herdr 0.9.3, worktrunk 0.79.0, Claude Code 2.1.312, Iosevka Term 34.8.1, Yaru GTK theme, dconf.

**Spec:** Taskwarrior task `f60e243e-83fa-45ef-9705-dc94cc3c0f0b` — its description and notes (`task rc.json.array=on f60e243e export`). No separate spec file.

## Global Constraints

- A Docker container, not a VM. A VM only if Task 2 or Task 4 proves niri cannot run in a container — then **stop and ask** the user before anything else.
- Image from `ubuntu:26.04`, pinned by digest, everything baked in, so a run installs and downloads nothing.
- **Taskwarrior is 3.5.0, compiled from the release tarball, and nothing else.** Never apt's `taskwarrior` (2.6.2) — not directly, not as a dependency. The image build fails if `dpkg -s taskwarrior` succeeds or `task --version` is not exactly `3.5.0`.
- **VS Code is excluded** (user's decision, 2026-10-28): no `code`, no stub. Everything else the machine has on the suites' PATH is in the image, at the version the machine runs.
- **Programs sit where they sit on the machine**, because the suites' behaviour depends on the folder: `herdr`, `wt`, `claude` and `task` in `~/.local/bin` (`task` a link into `~/.local/opt/taskwarrior-3.5.0`, `claude` a link into `~/.local/share/claude/versions`), ghostty and the apt programs in `/usr/bin`, and `~/.local/bin` on `PATH` before `/usr/local/bin` and `/usr/bin`. Nothing named `herdr` may be in `/usr/bin` or `/bin`: that is the e2e spawn PATH, and a herdr there makes e2e-panel and e2e-box skip their refine checks.
- **What `install.sh` links on the machine is linked in the container too**, from the snapshot to the same places. e2e-tag's refine checks need the refine mod.
- Code goes in as a snapshot of the current working tree, uncommitted edits included, copied in at start; it builds there with `NIRITASKS` set to that build; the cargo cache (the build directory) lives in a Docker volume.
- niri-tasks is compiled **inside** the container, never on the host and copied in: `cargo test` must compile there anyway, the host's `target/` cannot be reused at another path, and a binary built in the image links the image's libraries. Its dependencies are prebuilt in the image, so only the crate itself compiles on a run.
- Inside, a headless compositor runs an outer niri with two named workspaces, and `tests/all.sh` runs as it does now. `tests/all.sh`, `tests/e2e-*.sh`, `tests/lib/*`, `install.sh` and `src/` are **not** changed to suit the container. If a check can only pass by changing one of them, stop and tell the user.
- Nothing on the host is touched, and nothing outside it either: no bind mount of any host path, no host `XDG_RUNTIME_DIR`, Wayland, niri or D-Bus socket, **`--network none`**, `--cap-drop ALL`, `--security-opt no-new-privileges`, no `--privileged`. The only host writes are logs and failure artifacts under the checkout's own `target/container-tests/`, and Docker's own images and volumes, which the scripts prune.
- The wrapper is `bash tests/container.sh`: builds the image if needed, copies the tree in, runs `tests/all.sh`, saves the log, and can run in the background.
- Done when: from any worktree the wrapper runs all five suites in the container with zero skips and the same results as on the host, the host's windows, workspaces and task database are unchanged, and README documents it beside `tests/all.sh`.
- Out of scope: CI, and replacing `tests/all.sh` or `try.sh` on the host.
- Commits: Conventional Commits, `test(container): …`, `docs(readme): …`, subject ≤72 characters, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` trailer.

## The inventory (checked on 2026-10-28, so you need not)

What the suites reach for on this machine, where it comes from here, and what the image does about it. Sources read: `install.sh`, every `Command::new` and `on_path` in `src/` and `tests/*.rs`, every program the three e2e scripts, `tests/lib/*` and `herdr/task-docs/tests/` run, the `-sys` crates in `Cargo.lock` and `ldd ~/.cargo/bin/niritasks`, ubuntu-setup's `lib/manifest.sh`, `install.sh`, `configure.sh` and `home/`, and the machine's `~/.config/niri/config.kdl`.

### In the image

| What | Used by | On this machine | In the image |
|---|---|---|---|
| niri 26.04 | e2e suites (outer + nested), `tests/all.sh` checks | `niri 26.04ppa3`, `ppa:avengemedia/danklinux` (ubuntu-setup adds it; **not** in Ubuntu's archive) | same PPA, same package |
| Taskwarrior 3.5.0 | everything | compiled from the release tarball, `cmake --install` to `~/.local/opt/taskwarrior-3.5.0`, linked from `~/.local/bin/task` | compiled from the same tarball (sha256 pinned), same prefix, same link — so `nested-niri.sh`'s "taking out herdr's folder takes `task` with it" path runs as on the machine |
| herdr 0.9.3 | every herdr call the suites fake sits in front of it on PATH; `nested-niri.sh` strips its folder from every nested PATH and stops if it can still find it; `programs::tests::startup_commands_join_the_named_session_in_the_raw_folder` asserts the herdr argv only when it is on PATH | `~/.local/bin/herdr`, the static release binary | `herdr-linux-x86_64` v0.9.3 (sha256 pinned) into `~/.local/bin`. No herdr server of yours is reachable: no mounts, no network |
| worktrunk (`wt`) 0.79.0 | `src/work.rs` checks it is on PATH; e2e-tag and the cargo tests put a fake `wt` first | `~/.local/bin/wt`, ubuntu-setup's pinned musl tarball | the same tarball (sha256 pinned) into `~/.local/bin` |
| Claude Code 2.1.312 | `src/refine.rs` checks it is on PATH; no suite runs it (refines stop at the fake or missing herdr, and the e2e spawn PATH, `/usr/bin:/bin`, does not hold it on the machine either) | native install, `~/.local/bin/claude` → `~/.local/share/claude/versions/2.1.312` | the official installer pinned to 2.1.312 (it checks the download's checksum), auto-update off. Not logged in, and no network at run time |
| ghostty 1.3.1 | `programs::terminal_command`'s argv; on the e2e spawn PATH (`/usr/bin`) as on the machine | `ghostty 1.3.1ppa11`, `/usr/bin`, from the danklinux PPA (ubuntu-setup) | same PPA, same package |
| `install.sh`'s links | e2e-tag's `task refine` runs under the real `$HOME`, and `refine::launch` refuses unless `~/.local/share/niri-tasks/refine-mod/.claude-plugin/plugin.json` exists (`src/refine.rs:442`) | `install.sh` links `niri/niri-tasks.kdl`, seven skill files, `claude/refine-mod` and `claude/rules/niri-tasks.md` into `~/.config`, `~/.claude` and `~/.local/share` | `inside.sh` makes the same links from the snapshot at the start of every run, and stops if `install.sh` links a file its list lacks |
| outer niri's window handling | the nested niri's window must give it a 1600x1000 output, or `nested_start` dies on its size check | `prefer-no-csd`, `layout { border { off; } }` in `~/.config/niri/config.kdl` | the same two settings in `tests/container/niri.kdl`. Without `prefer-no-csd` the nested niri (a winit client) would draw its own title bar inside its window |
| rustc/cargo 1.95.0 | build, `cargo test` | rustup | rustup-init 1.29.2 (sha256 pinned), toolchain 1.95.0 minimal |
| the crate's dependencies, compiled | build, `cargo test` | `target/` of each checkout | compiled at image build into `/work/target` (dev profile, the path a run uses); seeded into each new build-directory volume |
| GTK 4.22 + gtk4-layer-shell 1.3 dev, libwayland-dev, pkg-config, build-essential | the crate's `-sys` crates (gtk4, gdk4-wayland, gtk4-layer-shell, wayland-sys, pango, graphene, gio) | apt (ubuntu-setup `APT_BUILD_PACKAGES`) | apt |
| Iosevka Term 34.8.1 | panel and box font, by name (`src/panel/style.rs:12`); covers every UI glyph (▶ ● ○ × … — · ─ ✓ ✕, checked with `fc-match …:charset=`) | `~/.local/share/fonts/iosevka-term/`, 54 TTFs from the release zip (ubuntu-setup `IOSEVKA_VERSION`) | same zip (sha256 pinned), `/usr/local/share/fonts/iosevka-term` |
| Noto Sans, Noto Color Emoji | GTK's default font `Adwaita Sans 11` is not installed, so fontconfig falls back to `NotoSans-Regular.ttf`; emoji to `NotoColorEmoji.ttf` | `fonts-noto-core`, `fonts-noto-color-emoji` | same packages |
| GTK settings | every GTK window the e2e suites draw | via the portal from gsettings: theme `Yaru-purple-dark`, color-scheme `prefer-dark`, icon theme `Yaru-purple-dark`, cursor `Adwaita`, font `Adwaita Sans 11`, antialiasing `grayscale`, hinting `slight`, rgba `rgb`, text scaling `1.0`, monospace `Ubuntu Sans Mono 11` | the same keys in a dconf system db, read through GSettings (no portal in the image); `yaru-theme-gtk`, `yaru-theme-icon`, `adwaita-icon-theme` |
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
| a uid-1000 user with `~/.taskrc` and `~/Projects/<folders>` | permission semantics; e2e-panel's project-list checks are skipped unless `~/Projects` holds a folder not named `e2e` (`tests/e2e-panel.sh:1008`, `:1056`) | `paul`, uid 1000; `~/.taskrc` has `data.location=$HOME/.task` | user `tester`, uid 1000 (the base image's `ubuntu` user removed); same `.taskrc` line; `~/Projects/alpha`, `~/Projects/beta` |
| session env | GTK, niri spawns | `XDG_CURRENT_DESKTOP=niri`, `XDG_SESSION_DESKTOP=niri`, `XDG_SESSION_TYPE=wayland` | set by `inside.sh` |

### Left out on purpose

| What | Why it is safe to leave out | What it changes |
|---|---|---|
| apt `taskwarrior` 2.6.2 (in ubuntu-setup's `APT_PACKAGES`) | the suites are written against 3.5.0 | — (the build refuses it) |
| VS Code (`code`) | user's decision | `programs::tests::the_editor_is_given_the_project_directory` returns early (passes without asserting), as on any machine without VS Code |
| gh | the project list's GitHub refresh, which needs your GitHub login and the network; without gh it keeps its cached list, which in the e2e sandbox is empty either way | nothing measured |
| plannotator-tui | the herdr plugin tests fake it | nothing |
| ffmpeg/ffplay, docker, nvidia-smi, curl's Kokoro use (`src/speak.rs`) | its unit tests only assert argv strings and run `cat` | nothing |
| mako, waybar, fuzzel, bubblewrap, socat, xdg-desktop-portal, the rest of your niri config and window rules | desktop pieces no suite reaches: notifications are stubbed, the nested niri has its own config, the refine sandbox is never started, GTK reads the same settings from dconf | nothing |
| the installed `niritasks`, its systemd unit, the herdr plugin registration (`herdr plugin link`) | the suites run the snapshot's build through `NIRITASKS`; `systemctl --user` missing makes `nested_service_untouched` compare `0` with `0`; the plugin tests run the plugin from the checkout with a fake herdr | nothing |

### Other facts

- In `ubuntu:26.04` with the PPA added, the image's whole apt list resolves (359 packages) and pulls in neither `taskwarrior`, `xdg-desktop-portal` nor `at-spi2-core` (checked with `apt-get install -s`). The rendering stack it installs is **the machine's, version for version**: libgtk-4-1 4.22.4+ds-0ubuntu0.1, libgtk4-layer-shell0 1.3.0-1, libfreetype6 2.14.2+dfsg-1ubuntu0.2, libharfbuzz0b 12.3.2-2, libpango-1.0-0 1.57.0-1, libcairo2 1.18.4-3, fontconfig-config 2.17.1-3ubuntu1, Mesa 26.0.8-1ubuntu0.3, niri 26.04ppa3, ghostty 1.3.1ppa11. `parity.sh` keeps watching this.
- Base image: `ubuntu:26.04@sha256:c098726dd9b9733f3e885f8de0ef375206a311b0a317eadc40d347b1115fb944` (created 2026-10-06), already pulled here.
- `docker build -f tests/container/Dockerfile -` with a tar of chosen files on stdin works here: the build sees only those files.
- Pins for the `~/.local/bin` programs: herdr `https://github.com/herdrdev/herdr/releases/download/v0.9.3/herdr-linux-x86_64`, sha256 `18a8dc65f1c2fa485884344356dea1cfd911c6f06cf46fa78e193f4087f4dba7` (from `https://herdr.dev/latest.json`'s `releases`); worktrunk `https://github.com/max-sixty/worktrunk/releases/download/v0.79.0/worktrunk-x86_64-unknown-linux-musl.tar.xz`, sha256 `b8c190b1d652370ef9f6b8f4694a2d6831b5b22f4b789f047612aad32764c6cf`, binary at `worktrunk-x86_64-unknown-linux-musl/wt` (ubuntu-setup `lib/manifest.sh`); Claude Code `curl -fsSL https://claude.ai/install.sh | bash -s 2.1.312`, run as the user, installs `~/.local/bin/claude` → `~/.local/share/claude/versions/2.1.312`.
- Taskwarrior 3.5.0: `https://github.com/GothenburgBitFactory/taskwarrior/releases/download/v3.5.0/task-3.5.0.tar.gz`, sha256 `9ea64b411f8314414f440ec765dfdf5a86c9f6159df47e2f60cff3db6b31157a`. CMake `3.24...4.0` policy range (apt's 4.2 is fine), C++17, libuuid, libsqlite3, Rust ≥ 1.91.1, network at configure (corrosion through git). Must be **installed** (`cmake --install`), or it cannot find `default.theme` (upstream #4174).
- rustup-init 1.29.2: `https://static.rust-lang.org/rustup/archive/1.29.2/x86_64-unknown-linux-gnu/rustup-init`, sha256 `5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4`.
- Iosevka Term 34.8.1: `https://github.com/be5invis/Iosevka/releases/download/v34.8.1/PkgTTF-IosevkaTerm-34.8.1.zip`, sha256 `c9929ef8c27e5469906468e378e11d3023690ae9cd411a4af3de78190b901075`, the same 54 files.
- The crate is one package with no build script, no dev-dependencies, no features and fixed `[lib]`/`[[bin]]` paths, so its dependencies prebuild with two stub source files and `cargo clean -p niri-tasks` afterwards; cargo-chef, the usual tool, earns its keep on workspaces and build scripts, which this crate has none of.
- `install.sh`'s file links are the lines that **start** with `link "$REPO/…"` (10 of them). `herdr plugin link "$REPO/herdr/task-docs"` also matches a looser grep and is not a file link.
- `refine::launch`'s socket fence (`src/refine.rs:351`) reads `/proc/net/unix`, which is per network namespace: in the container it sees only the container's sockets, all under `/tmp` (sway, both niris, the private D-Bus), which the fence hides. So the refine checks pass there for the same reason as on the machine.
- Spike, half done: in such an image `sway` (headless, pixman) runs an outer `niri` (`LIBGL_ALWAYS_SOFTWARE=1`) whose `niri msg workspaces` shows its two named workspaces, the first focused; a nested niri on it answers `niri msg action screenshot-screen` with a PNG of the configured `#406080`. Harmless log noise: `EGL_EXT_device_drm` unsupported, no login1/locale1, no xcursor theme, no Xwayland. Sway's default headless output is 1920x1080, which left the outer niri 1916x1053.
- The worktree's `.git` is a file, so the snapshot is `git ls-files --cached --others --exclude-standard`; no suite needs the checkout to be a repository.
- `tests/all.sh` defaults `NIRITASKS` to the installed binary; on the host, in a worktree, run it as `NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh` after `cargo build`.

## File Structure

- Create `tests/container/Dockerfile` — the image: the inventory above, pinned, with build-time guards on Taskwarrior, and the crate's dependencies prebuilt in `/work/target`.
- Create `tests/container/image.sh` — the image's name from the hash of its inputs, building it when missing (or on request), keeping the three newest; the build sees only those inputs, as a tar on stdin.
- Create `tests/container/dconf/00-machine` — the machine's GTK-relevant gsettings, as a dconf keyfile.
- Create `tests/container/check-image.sh` — proves the image is as designed; run against an image on stdin.
- Create `tests/container/parity.sh` — prints the versions and settings the image must share with the machine; run on both sides every run.
- Create `tests/container/gtk-settings.py` — prints what GTK sees (settings and renderer); run on the host and in the container, and diffed, while building this.
- Create `tests/container/sway.conf`, `tests/container/niri.kdl` — the headless compositor and the outer niri.
- Create `tests/container/inside.sh` — the in-container desktop, `install.sh`'s links, the build, then `tests/all.sh` or a given command; failure artifacts.
- Create `tests/container.sh` — the wrapper on the host.
- Modify `README.md` — "Running the tests": the command, a table row, a subsection. The Dockerfile and scripts point there for the why, not at this plan.

---

### Task 1: The image, and a check that proves it holds the inventory

Nothing is run in the image until this check passes: a missing piece shows up here as one line, not later as a failing pixel check.

**Files:**
- Create: `tests/container/check-image.sh`
- Create: `tests/container/dconf/00-machine`
- Create: `tests/container/Dockerfile`
- Create: `tests/container/image.sh`

**Interfaces:**
- Consumes: nothing.
- Produces: `bash tests/container/image.sh` — prints the image's name, `niritasks-tests:<12 hex>`, on stdout, building it first (build output on stderr) when no image has that name, or when `NIRITASKS_CONTAINER_REBUILD=1` (then with `--no-cache`, under the same name); after a build, removes `niritasks-tests` images beyond the three newest that no container uses; exits non-zero if the build fails. The name is a hash of `tests/container/Dockerfile`, `tests/container/dconf/00-machine`, `Cargo.toml` and `Cargo.lock`. The image's default user is `tester` (uid 1000, `HOME=/home/tester`), workdir `/work` (owned by `tester`, `/work/target` holding the crate's dependencies compiled in the dev profile and no niri-tasks artifacts), `CARGO_HOME=/opt/rust/cargo` (its registry holding every crate in `Cargo.lock`), `RUSTUP_HOME=/opt/rust/rustup`, `LANG=en_US.UTF-8`. `bash -s < tests/container/check-image.sh` run in it prints one `ok`/`MISSING` line per item and exits with the number missing.

- [ ] **Step 1: Write the check**

`tests/container/check-image.sh`:

```bash
#!/usr/bin/env bash
# Whether the test image is as designed: what the suites use on the
# machine, at the machine's versions, in the machine's folders. README's
# "Running the tests in a container" says why each piece is there. Run
# inside the image, from the host:
#
#   docker run --rm -i "$(bash tests/container/image.sh)" bash -s < tests/container/check-image.sh
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
         cargo rustc cc pkg-config task fc-match gsettings locale \
         herdr wt claude ghostty; do
    check "$p" command -v "$p"
done

echo "versions"
check "niri 26.04"          sh -c 'niri --version | grep -q "^niri 26\.04"'
check "task 3.5.0"          sh -c '[ "$(task --version)" = 3.5.0 ]'
check "rustc 1.95.0"        sh -c 'rustc --version | grep -q "^rustc 1\.95\.0 "'
check "no apt taskwarrior"  sh -c '! dpkg -s taskwarrior'
check "herdr 0.9.3"         sh -c '[ "$(herdr --version)" = "herdr 0.9.3" ]'
check "wt v0.79.0"          sh -c 'wt --version | grep -q "^wt v0\.79\.0"'
check "claude 2.1.312"      sh -c 'claude --version | grep -q "^2\.1\.312 "'
check "ghostty 1.3.1"       sh -c 'ghostty --version | grep -q "^Ghostty 1\.3\.1"'

echo "where they are, as on the machine"
for p in herdr wt claude task; do
    check "$p in ~/.local/bin" sh -c "[ \"\$(command -v $p)\" = \"\$HOME/.local/bin/$p\" ]"
done
check "task -> ~/.local/opt/taskwarrior-3.5.0" sh -c '[ "$(readlink -f "$HOME/.local/bin/task")" = "$HOME/.local/opt/taskwarrior-3.5.0/bin/task" ]'
check "ghostty in /usr/bin"   sh -c '[ "$(command -v ghostty)" = /usr/bin/ghostty ]'
check "no herdr on the e2e spawn PATH" sh -c '! PATH=/usr/bin:/bin command -v herdr'

echo "left out on purpose"
for p in code gh plannotator-tui; do
    check "no $p" sh -c "! command -v $p"
done
check "no desktop portal for GTK to wait on" \
    test ! -e /usr/share/dbus-1/services/org.freedesktop.portal.Desktop.service

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

echo "prebuilt dependencies"
check "gtk4 compiled in /work/target"  sh -c 'ls /work/target/debug/deps | grep -q "^libgtk4-"'
check "clap compiled in /work/target"  sh -c 'ls /work/target/debug/deps | grep -q "^libclap-"'
check "no niri-tasks artifacts"        sh -c '! ls /work/target/debug/.fingerprint | grep -q "^niri-tasks-"'
check "crates in the registry"         sh -c 'ls "$CARGO_HOME"/registry/cache/*/ | grep -q "^gtk4-"'

echo
echo "$missing missing"
exit "$missing"
```

- [ ] **Step 2: Run it against the bare base image, to see it fail**

Run: `docker run --rm -i ubuntu:26.04 bash -s < tests/container/check-image.sh; echo "exit: $?"`
Expected: mostly `MISSING` lines, ending `N missing` with N > 50, and a non-zero exit. (Only things like `tar`, `timeout`, `realpath`, `setsid`, `flock`, `no herdr on the e2e spawn PATH`, the `no …` lines and `no desktop portal` are `ok`.)

- [ ] **Step 3: Write the dconf keyfile**

`tests/container/dconf/00-machine` — the values are this machine's `gsettings get org.gnome.desktop.interface …` on 2026-10-28; GTK reads them through the portal there and straight from dconf here:

```
# The machine's GTK look, which every GTK window the e2e suites draw is
# drawn with. Copied from `gsettings get org.gnome.desktop.interface <key>`
# on the machine; tests/container/parity.sh compares the two at every run.
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
# the suites reach for on the machine — programs at the machine's versions
# and in its folders, fonts, GTK look, locale, user — and holds this crate's
# dependencies already compiled. README's "Running the tests in a
# container" says why each piece is here and what is left out;
# tests/container/check-image.sh proves the image is as designed.
# tests/container/image.sh builds it, with only this file, the dconf
# keyfile, Cargo.toml and Cargo.lock as its context, and names it after
# them, so a change to any of them builds a new image on the next run.

FROM ubuntu:26.04@sha256:c098726dd9b9733f3e885f8de0ef375206a311b0a317eadc40d347b1115fb944 AS base
# A failed download in a pipe fails the build, rather than run nothing.
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
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

# Taskwarrior 3.5.0, compiled from the release tarball as on the machine,
# into the prefix the machine uses. Never apt's taskwarrior, which is 2.6.2.
# Installed, not run from the build tree: 3.5.0 cannot find default.theme
# otherwise (upstream #4174).
FROM base AS taskwarrior
RUN apt-get update \
 && apt-get install -y --no-install-recommends cmake uuid-dev libsqlite3-dev \
 && rm -rf /var/lib/apt/lists/*
RUN curl -fsSLo /tmp/task.tar.gz https://github.com/GothenburgBitFactory/taskwarrior/releases/download/v3.5.0/task-3.5.0.tar.gz \
 && echo "9ea64b411f8314414f440ec765dfdf5a86c9f6159df47e2f60cff3db6b31157a  /tmp/task.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/task.tar.gz -C /tmp \
 && cmake -S /tmp/task-3.5.0 -B /tmp/task-build -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/home/tester/.local/opt/taskwarrior-3.5.0 \
 && cmake --build /tmp/task-build -j"$(nproc)" \
 && cmake --install /tmp/task-build

FROM base
ENV LANG=en_US.UTF-8 \
    DISABLE_AUTOUPDATER=1 \
    PATH=/home/tester/.local/bin:/opt/rust/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
# niri 26.04 is not in Ubuntu's archive: the machine's comes from the
# avengemedia/danklinux PPA, and so does this one, with ghostty. sway is
# only the headless compositor the outer niri runs in; Mesa is the software
# GL and Vulkan. The rest is what the suites use on the machine.
RUN apt-get update \
 && apt-get install -y --no-install-recommends software-properties-common gpg-agent \
 && add-apt-repository -y ppa:avengemedia/danklinux \
 && apt-get install -y --no-install-recommends \
      niri ghostty sway libgl1-mesa-dri libegl-mesa0 libegl1 mesa-vulkan-drivers libvulkan1 \
      dbus-daemon dbus-bin \
      libgtk-4-dev libgtk4-layer-shell-dev libwayland-dev libuuid1 libsqlite3-0 \
      wtype alacritty python3 python3-pil python3-gi gir1.2-gtk-4.0 \
      util-linux procps sqlite3 unzip xz-utils fzf xdg-utils libnotify-bin \
      fontconfig fonts-noto-core fonts-noto-color-emoji \
      adwaita-icon-theme yaru-theme-gtk yaru-theme-icon hicolor-icon-theme glycin-loaders librsvg2-common \
      dconf-gsettings-backend dconf-cli gsettings-desktop-schemas \
      locales tzdata \
 && rm -rf /var/lib/apt/lists/* \
 && locale-gen en_US.UTF-8

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
COPY tests/container/dconf/00-machine /etc/dconf/db/local.d/00-machine
RUN printf 'user-db:user\nsystem-db:local\n' > /etc/dconf/profile/user && dconf update

# A uid-1000 user, as on the machine, with what its home holds there: a
# .taskrc, and ~/Projects with folders in it, without which e2e-panel.sh
# skips its project-list checks. /work is made here, owned by it, so the
# build-directory volume mounted on /work/target starts out owned by it.
# The base image's own uid-1000 user, ubuntu, makes way for it.
RUN userdel -r ubuntu \
 && useradd -m -u 1000 -s /bin/bash tester \
 && printf 'data.location=$HOME/.task\n' > /home/tester/.taskrc \
 && mkdir -p /home/tester/Projects/alpha /home/tester/Projects/beta /home/tester/.local/bin \
             /work/target /opt/rust/cargo/registry \
 && chown -R tester:tester /home/tester /work /opt/rust
USER tester
WORKDIR /work

# What the machine has in ~/.local/bin, where it has it. The folder
# matters: tests/lib/nested-niri.sh takes herdr's folder off every nested
# PATH, and `task` beside it with it, and a herdr in /usr/bin would make the
# e2e suites skip their refine checks.
COPY --from=taskwarrior --chown=tester:tester /home/tester/.local/opt/taskwarrior-3.5.0 /home/tester/.local/opt/taskwarrior-3.5.0
RUN ln -s ../opt/taskwarrior-3.5.0/bin/task /home/tester/.local/bin/task \
 && ! dpkg -s taskwarrior >/dev/null 2>&1 && [ "$(task --version)" = 3.5.0 ]
RUN curl -fsSLo /home/tester/.local/bin/herdr https://github.com/herdrdev/herdr/releases/download/v0.9.3/herdr-linux-x86_64 \
 && echo "18a8dc65f1c2fa485884344356dea1cfd911c6f06cf46fa78e193f4087f4dba7  /home/tester/.local/bin/herdr" | sha256sum -c - \
 && chmod +x /home/tester/.local/bin/herdr
RUN curl -fsSLo /tmp/wt.tar.xz https://github.com/max-sixty/worktrunk/releases/download/v0.79.0/worktrunk-x86_64-unknown-linux-musl.tar.xz \
 && echo "b8c190b1d652370ef9f6b8f4694a2d6831b5b22f4b789f047612aad32764c6cf  /tmp/wt.tar.xz" | sha256sum -c - \
 && tar -xJf /tmp/wt.tar.xz -C /tmp worktrunk-x86_64-unknown-linux-musl/wt \
 && install -m 755 /tmp/worktrunk-x86_64-unknown-linux-musl/wt /home/tester/.local/bin/wt \
 && rm -rf /tmp/wt.tar.xz /tmp/worktrunk-x86_64-unknown-linux-musl
# Claude Code's own installer, pinned to the machine's version; it checks the
# download against its release manifest. Never logged in here.
RUN curl -fsSL https://claude.ai/install.sh | bash -s 2.1.312 \
 && [ "$(readlink /home/tester/.local/bin/claude)" = /home/tester/.local/share/claude/versions/2.1.312 ]

# This crate's dependencies, compiled as inside.sh compiles (cargo build,
# dev profile, in /work), so their paths and hashes are the ones a run
# looks for; and every crate in Cargo.lock in the registry, so a run needs
# no network. Docker copies /work/target into each new, empty build-
# directory volume mounted over it, so a checkout's first run compiles only
# niri-tasks. The crate itself is two empty stand-ins here, cleaned away
# afterwards: left behind, cargo would take their artifacts as up to date
# against the real sources, whose mtimes are older than this build. cargo
# test reuses these too: its profile inherits dev's. Last, because
# Cargo.lock changes more often than anything above.
COPY --chown=tester:tester Cargo.toml Cargo.lock /work/
RUN mkdir -p src && printf 'fn main() {}\n' > src/main.rs && : > src/lib.rs \
 && cargo build --locked \
 && cargo clean --locked -p niri-tasks \
 && rm -rf src Cargo.toml Cargo.lock
```

Notes for the implementer:
- `unzip -j … '*.ttf'` flattens any subfolder; the check wants exactly 54 TTFs.
- The base stage's `git` is what Taskwarrior's configure uses to fetch corrosion.

- [ ] **Step 5: Write image.sh**

`tests/container/image.sh`:

```bash
#!/usr/bin/env bash
# The test image's name, building it first if there is none. tests/container.sh
# runs it; so can you, to build the image or check what is in it:
#
#   image=$(bash tests/container/image.sh)
#   docker run --rm -i "$image" bash -s < tests/container/check-image.sh
#
# The image is named after everything it is built from — the Dockerfile, the
# dconf keyfile, Cargo.toml and Cargo.lock — so a change to any of them
# builds a new one, and nothing else does. Those four files are all the build
# sees: they go to docker build as a tar on stdin, so the rest of the
# checkout is never sent. The name is printed on stdout, the build on stderr.
#
# NIRITASKS_CONTAINER_REBUILD=1 builds it again under the same name, without
# Docker's cache, so apt brings it up to what the machine has now: what to do
# when tests/container/parity.sh says the machine has moved on.
#
# After a build, the three newest test images are kept — a worktree on
# another Cargo.lock needs its own — and older ones go, unless a container
# still uses one.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

INPUTS=(tests/container/Dockerfile tests/container/dconf/00-machine Cargo.toml Cargo.lock)
IMAGE="niritasks-tests:$(sha256sum "${INPUTS[@]}" | sha256sum | cut -c1-12)"

if [ -n "${NIRITASKS_CONTAINER_REBUILD:-}" ] || ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
    fresh=()
    [ -n "${NIRITASKS_CONTAINER_REBUILD:-}" ] && fresh=(--no-cache)
    echo "building $IMAGE: once, and again only when one of ${INPUTS[*]} changes" >&2
    tar -cf - "${INPUTS[@]}" |
        docker build "${fresh[@]}" -t "$IMAGE" -f tests/container/Dockerfile - >&2
    # Newest first, as docker lists them; past the third image, every tag goes.
    docker image ls niritasks-tests --format '{{.ID}} {{.Repository}}:{{.Tag}}' |
        awk '!($1 in seen) { seen[$1]; n++ } n > 3 { print $2 }' |
        xargs -r docker image rm >/dev/null 2>&1 || true
fi
echo "$IMAGE"
```

- [ ] **Step 6: Build the image and run the check**

```bash
chmod +x tests/container/image.sh tests/container/check-image.sh
IMAGE=$(bash tests/container/image.sh) && docker tag "$IMAGE" niritasks-tests:dev
docker run --rm -i niritasks-tests:dev bash -s < tests/container/check-image.sh; echo "exit: $?"
```

The first build takes several minutes: Taskwarrior compiles, the apt list is about 360 packages, Claude Code is a ~200MB download, and the crate's dependencies compile (gtk4 and its bindings). Expected: every line `ok`, `0 missing`, `exit: 0`. Then run `bash tests/container/image.sh` again: it prints the same name at once, with no build. `niritasks-tests:dev` is a second name for the same image, for the one-off runs in Task 2.

A `MISSING` line is fixed in the Dockerfile (or the keyfile), then rebuild and rerun — never by loosening the check. If a value can't be matched (say `gsettings get` prints a font name differently), compare against `gsettings get org.gnome.desktop.interface <key>` on the host, and stop and ask if the host's own value is now different from the plan's.

- [ ] **Step 7: Commit**

```bash
git add tests/container
git commit -m "test(container): add the test image and a check of what it holds

A ubuntu:26.04 image with what the suites use on the machine: niri 26.04
and ghostty from the same PPA, Taskwarrior 3.5.0 compiled from its tarball
(never apt's 2.6), herdr, worktrunk and Claude Code in ~/.local/bin as
there, rustc 1.95.0, Iosevka Term 34.8.1, the machine's GTK look as a
dconf database, en_US.UTF-8, a uid-1000 user, and the crate's
dependencies prebuilt. check-image.sh lists each piece as ok or
missing; image.sh names the image after its inputs and builds it when
they change.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The desktop in the container, matching the machine's

The first half of the task's spike gate: niri runs in the container, a nested niri parks on it at the size the e2e suites demand, and a GTK window there is drawn with the machine's settings and renderer. Still no suite runs, and nothing is compiled.

**Files:**
- Create: `tests/container/sway.conf`
- Create: `tests/container/niri.kdl`
- Create: `tests/container/inside.sh`
- Create: `tests/container/parity.sh`
- Create: `tests/container/gtk-settings.py`

**Interfaces:**
- Consumes: the image from Task 1 (`niritasks-tests:dev`), user `tester`, workdir `/work`.
- Produces:
  - `bash tests/container/inside.sh [--no-build] [command...]`, run from `/work` (the snapshot) under `dbus-run-session`: reports parity against `NIRITASKS_HOST_PARITY` when set; makes `install.sh`'s links (stops if `install.sh` links a file it does not); stands up sway and the outer niri and exports `NIRI_SOCKET`, `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`; exports `CARGO_NET_OFFLINE=true` and `NIRITASKS_E2E_KEEP=1`; builds with `cargo build --locked` and exports `NIRITASKS=/work/target/debug/niritasks` (skipped with `--no-build`); runs the command (default `bash tests/all.sh`) and exits with its status. On a non-zero status it first writes `/tmp/artifacts.tar` (regular files only: `/tmp/sway.log`, `/tmp/niri.log`, and every kept `/tmp/tmp.*` e2e sandbox). Exits 1 with `container: …` on stderr when the desktop, the links or the build cannot be made.
  - `bash tests/container/parity.sh` — one `name value` line per package version, program version, font version and GTK setting the image must share with the machine; same output on both sides when they match.
  - `python3 tests/container/gtk-settings.py` — `name value` lines, the last `renderer <GskRenderer class>`.

- [ ] **Step 1: Write the two probes, and record the host's answers**

`tests/container/parity.sh`:

```bash
#!/usr/bin/env bash
# What the test image must share with the machine for the suites to see the
# same thing there as here: the drawing stack's versions, the programs the
# suites run, the font, and the GTK settings. One "name value" line each.
# tests/container.sh runs it on the machine, inside.sh runs it in the
# container, and a difference is printed at the top of the run: the image
# was built from an older machine, and NIRITASKS_CONTAINER_REBUILD=1 brings
# it up to date. Mesa is not here: the machine draws with its GPU.
set -u

for p in niri ghostty libgtk-4-1 libgtk4-layer-shell0 libpango-1.0-0 libcairo2 \
         libharfbuzz0b libfreetype6 fontconfig-config fonts-noto-core yaru-theme-gtk; do
    echo "$p $(dpkg-query -W -f '${Version}' "$p" 2>/dev/null || echo missing)"
done
for c in task herdr wt claude rustc; do
    echo "$c $("$c" --version 2>/dev/null | head -n 1 || echo missing)"
done
echo "iosevka-term-extended $(fc-match -f '%{fontversion}' 'Iosevka Term Extended' 2>/dev/null || echo missing)"
for k in color-scheme gtk-theme icon-theme cursor-theme font-name monospace-font-name \
         font-antialiasing font-hinting font-rgba-order text-scaling-factor; do
    echo "gsettings.$k $(gsettings get org.gnome.desktop.interface "$k" 2>/dev/null || echo missing)"
done
```

`tests/container/gtk-settings.py`:

```python
"""What GTK sees: the settings that decide how a window is drawn, and the
renderer it draws with. Run on the machine and in the test container
(tests/container/inside.sh) and diff the two; they should be the same.
tests/container/parity.sh checks the inputs at every run; this checks what
GTK makes of them, which needs a display.

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

Run on the host:

```bash
bash tests/container/parity.sh | tee target/parity-host.txt
python3 tests/container/gtk-settings.py | tee target/gtk-host.txt
```

Expected for `gtk-settings.py` (2026-10-28; a window flashes on your workspace for half a second):

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

and for `parity.sh`, versions matching the "Other facts" list, `task 3.5.0`, `herdr 0.9.3`, `wt v0.79.0`, `claude 2.1.312 (Claude Code)`, `rustc 1.95.0 (…)`, `iosevka-term-extended 2233473`, and the gsettings values of the dconf keyfile.

- [ ] **Step 2: Write the compositor and outer niri configs**

`tests/container/sway.conf`:

```
# The headless compositor the container's outer niri runs in
# (tests/container/inside.sh). niri needs one: its own headless backend is
# only for niri's tests. One output big enough that the 1600x1000 nested
# niri the e2e tests park fits on the outer niri with room to spare. A
# headless output advertises no modes, hence --custom.
output * mode --custom 2560x1440
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
// niri keeps after them.
//
// prefer-no-csd and no border, as in the machine's config: the nested niri
// is a winit client, and asked to draw its own decorations it would put a
// title bar inside its window, leaving its output short of the 1600x1000
// tests/lib/nested-niri.sh insists on.
//
// The startup line is how inside.sh learns this niri's sockets.
prefer-no-csd
hotkey-overlay { skip-at-startup; }
xwayland-satellite { off; }
animations { off; }
layout {
    border { off; }
}
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
# It stands up what tests/all.sh expects to find on the machine, then gets
# out of the way: the links install.sh makes, a headless sway, an outer niri
# as its one window with two named workspaces, and the session variables
# niri's session sets. tests/all.sh then runs as it does on the desktop, and
# its e2e suites nest their own niri inside this one. Nothing here is the
# machine's: no socket, file or display of the host reaches the container,
# and it has no network.
#
# It builds the snapshot first and points NIRITASKS at that build: there is
# no installed niritasks in here to test by mistake. When the command fails,
# the run's logs and the e2e sandboxes it kept (frames included) go into
# /tmp/artifacts.tar for tests/container.sh to copy out.
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
# Every crate is in the image's registry; with no network, say so at once
# rather than time out.
export CARGO_NET_OFFLINE=true
# The e2e suites keep their sandboxes, frames and logs, instead of deleting
# them: in here they go with the container, unless a failure has them copied
# out.
export NIRITASKS_E2E_KEEP=1

# Whether the image still matches the machine. A difference is worth knowing
# before reading the results, but is not a failure.
if [ -n "${NIRITASKS_HOST_PARITY:-}" ]; then
    if drift=$(diff <(printf '%s\n' "$NIRITASKS_HOST_PARITY") <(bash tests/container/parity.sh)); then
        echo "parity: the image matches this machine"
    else
        echo "parity: the image differs from this machine (< machine, > image):"
        printf '%s\n' "$drift" | grep '^[<>]' | sed 's/^/  /'
        echo "  rebuild it with: NIRITASKS_CONTAINER_REBUILD=1 bash tests/container.sh"
    fi
fi

# What install.sh links on the machine, from this checkout, to the same
# places. e2e-tag.sh's refine checks need the refine mod: refine::launch
# refuses to start without it. The rest make this home look like the
# machine's. install.sh is the authority: a file it links that is not listed
# here stops the run, so the two cannot drift apart quietly.
LINKS=(
    "niri/niri-tasks.kdl                             .config/niri/niri-tasks.kdl"
    ".claude/skills/workspace-tasks/SKILL.md         .claude/skills/workspace-tasks/SKILL.md"
    ".claude/skills/refine-task/SKILL.md             .claude/skills/refine-task/SKILL.md"
    ".claude/skills/refine-task/report-catalogue.md  .claude/skills/refine-task/report-catalogue.md"
    ".claude/skills/report-task/SKILL.md             .claude/skills/report-task/SKILL.md"
    ".claude/skills/finish-worktree/SKILL.md         .claude/skills/finish-worktree/SKILL.md"
    ".claude/skills/reject-worktree/SKILL.md         .claude/skills/reject-worktree/SKILL.md"
    ".claude/skills/private-github-repo/SKILL.md     .claude/skills/private-github-repo/SKILL.md"
    "claude/refine-mod                               .local/share/niri-tasks/refine-mod"
    "claude/rules/niri-tasks.md                      .claude/rules/niri-tasks.md"
)
for entry in "${LINKS[@]}"; do
    read -r src dst <<< "$entry"
    [ -e "$src" ] || die "install.sh's $src is not in this checkout"
    mkdir -p "$HOME/$(dirname "$dst")"
    ln -sfn "$PWD/$src" "$HOME/$dst"
done
# Only lines that start with `link`: `herdr plugin link "$REPO/…"` is a
# herdr registration, not a file link.
unlisted=""
while read -r src; do
    printf '%s\n' "${LINKS[@]}" | awk '{ print $1 }' | grep -qxF "$src" || unlisted="$unlisted $src"
done < <(grep -oE '^link "\$REPO/[^"]*"' install.sh | sed 's/^link "\$REPO\///; s/"$//')
[ -z "$unlisted" ] || die "install.sh links what inside.sh does not:$unlisted — add them to LINKS"

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
status=$?
if [ "$status" -ne 0 ]; then
    # Regular files only: the sandboxes also hold sockets, which do not copy.
    (cd /tmp && find sway.log niri.log tmp.* -type f 2>/dev/null) > /tmp/artifacts.list
    tar -C /tmp -cf /tmp/artifacts.tar -T /tmp/artifacts.list 2>/dev/null
fi
exit "$status"
```

- [ ] **Step 4: Run the desktop, a parked nested niri and both probes in the container, and diff**

A one-off run, the way the wrapper (Task 3) will do it, minus the volume:

```bash
chmod +x tests/container/inside.sh tests/container/parity.sh
git ls-files -z --cached --others --exclude-standard |
    while IFS= read -r -d '' f; do { [ -e "$f" ] || [ -L "$f" ]; } && printf '%s\0' "$f"; done |
    tar --null -T - -cf - |
    docker run --rm -i --init --network none --cap-drop ALL --security-opt no-new-privileges \
        -e TZ="$(cat /etc/timezone)" -e NIRITASKS_HOST_PARITY="$(cat target/parity-host.txt)" \
        niritasks-tests:dev \
        bash -c 'tar -xf - -C /work && exec dbus-run-session -- bash /work/tests/container/inside.sh --no-build bash -c "
            niri msg outputs
            ls -l ~/.local/share/niri-tasks/refine-mod/.claude-plugin/plugin.json
            python3 tests/container/gtk-settings.py
            NIRITASKS=true; . tests/lib/nested-niri.sh && nested_start && summary"' \
    | tee target/container-desktop.txt
diff <(grep -E '^(gtk-|renderer )' target/gtk-host.txt) <(grep -E '^(gtk-|renderer )' target/container-desktop.txt) && echo "GTK: same"
```

`NIRITASKS=true` stands in for a build: `nested_start` only needs some program by that name, and this step compiles nothing.

Expected, in order:
- `parity: the image matches this machine`.
- `niri msg workspaces` listing `alpha` (focused) and `beta` and an empty third; `niri msg outputs` with a logical size of at least 1600x1000 (about 2560x1440).
- the refine mod's `plugin.json` listed through its link.
- `PASS  no HERDR_* reaches the nested niri or what runs in it`, `PASS  the nested niri is parked unfocused on a spare workspace, 1600x1000`, `passed: 2   failed: 0   skipped: 0`. That is the e2e harness's own parking, size and herdr-stripping logic, run with a real herdr in `~/.local/bin` as on the machine.
- `GTK: same`.

If something differs, one thing at a time:
- `parity: the image differs`: the printed lines say what. A version line means the machine updated after the image was built: `NIRITASKS_CONTAINER_REBUILD=1 bash tests/container/image.sh` and retag `:dev`. A gsettings line means the dconf keyfile no longer matches the machine: stop and ask whether the machine's setting is meant.
- `the nested niri's output is …, not 1600x1000`: the outer niri gave the nested window decorations or a border. Check `prefer-no-csd` and `border { off; }` are in `tests/container/niri.kdl` and that `/tmp/niri.log` has no config error.
- `gtk-theme-name`, `gtk-interface-color-scheme`, `gtk-font-name`, `gtk-xft-*`, `gtk-icon-theme-name`, `gtk-cursor-theme-name`: GTK is not reading the dconf database. Check `GSETTINGS_BACKEND` is unset and Task 1's check passes; GTK on Wayland reads GSettings when there is no portal. If it still differs, stop and ask.
- `gtk-xft-dpi`: the outer output's scale is not 1. Add `output "winit" { scale 1; }` to `tests/container/niri.kdl`.
- `renderer GLRenderer` or `CairoRenderer` instead of `VulkanRenderer`: lavapipe is not usable over Wayland here. Rerun with `GSK_DEBUG=renderer` to see why; if lavapipe cannot be made to work, accept `GLRenderer` (llvmpipe), say so in a comment in `inside.sh`, and tell the user the renderer differs.
- The outer output smaller than 1600x1000: sway ignored the mode; check `/tmp/sway.log` (add `cat /tmp/sway.log` to the command above).

**Stop and ask the user** if the outer niri does not start at all, or a nested niri cannot be parked at 1600x1000: that is the task's "niri cannot run in a container" gate.

- [ ] **Step 5: Check the drift guard on install.sh's links**

Run a copy of the snapshot with one link removed from `LINKS`, to see the guard fire:

```bash
git ls-files -z --cached --others --exclude-standard |
    while IFS= read -r -d '' f; do { [ -e "$f" ] || [ -L "$f" ]; } && printf '%s\0' "$f"; done |
    tar --null -T - -cf - |
    docker run --rm -i --network none niritasks-tests:dev \
        bash -c 'tar -xf - -C /work && sed -i "/claude\/rules\/niri-tasks.md/d" /work/tests/container/inside.sh && bash /work/tests/container/inside.sh --no-build true'; echo "exit: $?"
```

Expected: `container: install.sh links what inside.sh does not: claude/rules/niri-tasks.md — add them to LINKS`, `exit: 1`.

- [ ] **Step 6: Commit**

```bash
git add tests/container
git commit -m "test(container): stand up a desktop like the machine's in the image

inside.sh makes install.sh's links, starts a headless sway and an outer
niri set up like the machine's, builds the snapshot offline and runs
tests/all.sh or a given command, keeping a failed run's logs and frames.
parity.sh and gtk-settings.py give the same answers on the machine and
in the container, and a nested niri parks there at 1600x1000.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The wrapper, tests/container.sh

**Files:**
- Create: `tests/container.sh`

**Interfaces:**
- Consumes: `bash tests/container/image.sh` (Task 1) for the image's name, honouring `NIRITASKS_CONTAINER_REBUILD`; `bash tests/container/parity.sh` on the host (Task 2); `bash /work/tests/container/inside.sh [command...]` (Task 2), run under `dbus-run-session` as `tester` with `/work/target` mounted and `NIRITASKS_HOST_PARITY` and `TZ` set; `/tmp/artifacts.tar` left by a failed run.
- Produces: `bash tests/container.sh [command...]` — exits with the in-container command's status (default `bash tests/all.sh`); 1 when Docker is missing or unreachable, the image cannot be built, or a run of the same checkout is already going; 130 when interrupted. Writes the whole run to `target/container-tests/<stamp>.log` (`<stamp>` = UTC time and pid), `latest.log` pointing at it, and on a failure (not an interrupt) the run's logs and kept e2e sandboxes to `target/container-tests/<stamp>-artifacts/`. Containers carry `niritasks.tests.root=<checkout path>`. Build-directory volumes are named `niritasks-tests-target-<path hash>-<image id>` and labelled `niritasks.tests=1`, `niritasks.tests.root`, `niritasks.tests.image`; each run first removes volumes whose checkout is gone, whose image is gone, or that belong to this checkout and an older image.

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
# container gets no mount, socket or display of the machine's and no
# network, only the files of this checkout, copied in on stdin.
#
# The copy is what git would see of the working tree: tracked files as they
# are now, edited or not, and untracked ones that are not ignored. Not
# target/, not .git.
#
# The image (tests/container/image.sh) carries the crate's dependencies
# already compiled. Each checkout's build directory is a Docker volume of
# its own, filled from the image when it is new, so a checkout's first run
# compiles only niri-tasks and later runs only what changed. A volume
# belongs to one image: a new image starts a new one, and the old one goes,
# as do those of checkouts since removed. Two runs of the same checkout at
# once would share a build directory, so the second refuses; runs of
# different worktrees go side by side.
#
# Each run first says whether the image still matches the machine
# (tests/container/parity.sh); NIRITASKS_CONTAINER_REBUILD=1 rebuilds it
# when not. Everything the run prints is saved in target/container-tests/,
# latest.log the newest; a failed run's logs and the e2e suites' frames go
# beside it, in <stamp>-artifacts/. It exits as tests/all.sh does: non-zero
# only when a suite that ran said no.
set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || exit 1
cd "$ROOT" || exit 1

command -v docker >/dev/null || { echo "docker is required" >&2; exit 1; }
docker info >/dev/null 2>&1 || { echo "docker is not running, or this user cannot reach it" >&2; exit 1; }

KEY=$(printf '%s' "$ROOT" | sha256sum | cut -c1-12)
# The machine's timezone, so a due date means the same day in here.
TZ_NAME=$(cat /etc/timezone 2>/dev/null || timedatectl show -p Timezone --value 2>/dev/null || echo UTC)

running=$(docker ps --filter "label=niritasks.tests.root=$ROOT" --format '{{.Names}}')
if [ -n "$running" ]; then
    echo "a container run of $ROOT is already going ($running), and two would share one build directory." >&2
    echo "If it is stuck: docker rm -f $running" >&2
    exit 1
fi

LOGS="$ROOT/target/container-tests"
mkdir -p "$LOGS" || exit 1
STAMP="$(date -u +%Y%m%dT%H%M%SZ)-$$"
LOG="$LOGS/$STAMP.log"
ln -sfn "$(basename "$LOG")" "$LOGS/latest.log"
exec > >(tee "$LOG") 2>&1
TEE=$!

NAME=""
# On any exit: a failed run's artifacts copied out, the container removed,
# and the log let finish before the shell returns.
finish() {
    local status=$?
    if [ -n "$NAME" ]; then
        if [ "$status" -ne 0 ] && [ "$status" -ne 130 ] &&
            docker cp "$NAME:/tmp/artifacts.tar" "$LOGS/$STAMP-artifacts.tar" >/dev/null 2>&1; then
            mkdir -p "$LOGS/$STAMP-artifacts" &&
                tar -xf "$LOGS/$STAMP-artifacts.tar" -C "$LOGS/$STAMP-artifacts" &&
                rm -f "$LOGS/$STAMP-artifacts.tar" &&
                echo "kept  $LOGS/$STAMP-artifacts (logs and the e2e suites' frames)"
        fi
        docker rm -f "$NAME" >/dev/null 2>&1
    fi
    echo
    echo "log   $LOG"
    exec >&- 2>&-
    wait "$TEE" 2>/dev/null
    exit "$status"
}
trap finish EXIT
trap 'exit 130' INT TERM

echo "repo  $ROOT"
echo "log   $LOG"

IMAGE=$(bash tests/container/image.sh) || exit 1
IMAGE_ID=$(docker image inspect -f '{{.Id}}' "$IMAGE" | sed 's/^sha256://' | cut -c1-12)
echo "image $IMAGE ($IMAGE_ID)"

# Build directories nothing will use again: their checkout is gone, their
# image is gone, or they are this checkout's from an older image. One in use
# by another run cannot be removed and is left for a later run.
docker volume ls -q --filter label=niritasks.tests.root | while read -r volume; do
    root=$(docker volume inspect -f '{{index .Labels "niritasks.tests.root"}}' "$volume")
    image=$(docker volume inspect -f '{{index .Labels "niritasks.tests.image"}}' "$volume")
    if [ ! -d "$root" ] || ! docker image inspect "$image" >/dev/null 2>&1 ||
        { [ "$root" = "$ROOT" ] && [ "$image" != "$IMAGE_ID" ]; }; then
        docker volume rm "$volume" >/dev/null 2>&1 || true
    fi
done
VOLUME="niritasks-tests-target-$KEY-$IMAGE_ID"
docker volume inspect "$VOLUME" >/dev/null 2>&1 ||
    docker volume create --label niritasks.tests=1 --label "niritasks.tests.root=$ROOT" \
        --label "niritasks.tests.image=$IMAGE_ID" "$VOLUME" >/dev/null || exit 1

# The working tree as git sees it, NUL-separated. A tracked file deleted but
# not yet committed is left out rather than failing the copy.
snapshot() {
    git ls-files -z --cached --others --exclude-standard |
        while IFS= read -r -d '' f; do
            { [ -e "$f" ] || [ -L "$f" ]; } && printf '%s\0' "$f"
        done |
        tar --null -T - -cf -
}

# Waited for in the background: bash runs a trap only once the foreground
# command is over, and docker run is not over until the suites are. $! is
# the pipeline's last command, docker run, whose status is the run's.
NAME="niritasks-tests-$KEY-$$"
snapshot | docker run -i --init --name "$NAME" \
    --label "niritasks.tests.root=$ROOT" \
    --network none --cap-drop ALL --security-opt no-new-privileges \
    --shm-size 512m -e TZ="$TZ_NAME" \
    -e NIRITASKS_HOST_PARITY="$(bash tests/container/parity.sh)" \
    -v "$VOLUME:/work/target" \
    "$IMAGE" \
    bash -c 'tar -xf - -C /work && exec dbus-run-session -- bash /work/tests/container/inside.sh "$@"' inside "$@" &
wait "$!"
exit $?
```

Notes for the implementer:
- No `-t`: a TTY would stop it running in the background or from an agent's shell, and would mangle the tar on stdin.
- No `--rm`: `finish` removes the container itself, after copying a failed run's artifacts out of it.
- `"$@"` after `inside` passes the optional command through `bash -c` intact (`inside` becomes `$0`).
- `TEE=$!` is the process substitution's pid (bash ≥ 4.4); `wait "$TEE"` (bash ≥ 5.1) lets the last lines reach the log before the shell returns.

- [ ] **Step 3: Run the desktop alone through it**

Run: `chmod +x tests/container.sh && bash tests/container.sh --no-build niri msg workspaces; echo "exit: $?"`
Expected: `repo`, `log`, `image niritasks-tests:<hash> (<id>)` lines, `parity: the image matches this machine`, then `niri msg workspaces` twice (once from `inside.sh`'s own check) showing `alpha` and `beta`, `exit: 0`.

Run: `ls -l target/container-tests/ && tail -n 2 target/container-tests/latest.log && docker volume ls --filter label=niritasks.tests.root="$PWD"`
Expected: one `<stamp>.log`, `latest.log -> <that file>`, the log ending with the `log` line, and one volume `niritasks-tests-target-<hash>-<id>`.

- [ ] **Step 4: Check the build, the cache and the exit status**

Run: `time bash tests/container.sh true; echo "exit: $?"` twice.
Expected: the first prints `Compiling niri-tasks` and **no other** `Compiling` or `Downloading` line — the dependencies came with the volume, seeded from the image in Step 3 — in about the time an edit-and-rebuild takes on the host; the second prints `Finished` with nothing compiled, in seconds; both `exit: 0`. If another crate compiles, the volume was not seeded from the image: check it was new when Step 3 created it, and check the image with `check-image.sh`.

- [ ] **Step 5: Check the isolation and the failure artifacts**

Run: `bash tests/container.sh --no-build sh -c 'curl -sS --max-time 5 https://github.com >/dev/null'; echo "exit: $?"`
Expected: curl's `Could not resolve host: github.com` and a non-zero exit — no network in the container.

Run: `bash tests/container.sh --no-build false; echo "exit: $?"; ls target/container-tests/*-artifacts/ | tail -n 3`
Expected: `kept  …/<stamp>-artifacts (logs and the e2e suites' frames)`, `exit: 1`, and `niri.log` and `sway.log` in the newest artifacts folder.

Run: `docker ps -a --filter label=niritasks.tests.root="$PWD" -q | wc -l`
Expected: `0` — no container left behind after either run.

- [ ] **Step 6: Check the refusals, the interrupt and the pruning**

```bash
bash tests/container.sh --no-build sleep 60 >/dev/null 2>&1 &
sleep 15; bash tests/container.sh --no-build true; echo "exit: $?"; wait
```

Expected: `a container run of /home/…/task-test-run-the-test-suite-in-a-docker-f60e243e is already going (niritasks-tests-…), and two would share one build directory.`, the `docker rm -f` hint, `exit: 1`.

```bash
bash tests/container.sh --no-build sleep 300 >/dev/null 2>&1 & pid=$!
sleep 20; kill -TERM "$pid"; wait "$pid"; echo "exit: $?"
docker ps -aq --filter label=niritasks.tests.root="$PWD" | wc -l
```

Expected: `exit: 130`, then `0` — the container went at once, not after 300s. (TERM, not INT: a background job of a non-interactive shell ignores INT; Ctrl-C on a foreground run reaches the same trap.)

```bash
docker volume create --label niritasks.tests=1 --label niritasks.tests.root=/nonexistent/checkout \
    --label niritasks.tests.image=000000000000 niritasks-tests-target-prunetest >/dev/null
bash tests/container.sh --no-build true >/dev/null; docker volume inspect niritasks-tests-target-prunetest >/dev/null 2>&1 && echo "still there" || echo "pruned"
```

Expected: `pruned`.

- [ ] **Step 7: Commit**

```bash
git add tests/container.sh
git commit -m "test(container): add tests/container.sh to run the suites in Docker

Builds the image when its inputs change, copies the working tree in on
stdin, runs tests/all.sh with no network and no host mounts, keeps the
log in target/container-tests/ and a failed run's logs and frames
beside it. Each checkout's build directory is a volume seeded from the
image's prebuilt dependencies, and volumes nothing will use again are
removed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Every suite in the container, e2e-panel.sh first

The second half of the spike gate, then the rest. Each suite is run alone first, so a failure has one place to come from.

**Files:**
- Modify only if a suite fails or skips: `tests/container/Dockerfile`, `tests/container/dconf/00-machine`, `tests/container/inside.sh`, `tests/container/niri.kdl`, `tests/container/sway.conf` (and `tests/container/check-image.sh` and `parity.sh` with any inventory change)

**Interfaces:**
- Consumes: `bash tests/container.sh [command...]` (Task 3) and its `<stamp>-artifacts/` folders.
- Produces: `target/container-tests/host.log` — the host's own `tests/all.sh` run, for Task 5.

- [ ] **Step 1: Record the host's results**

Tell the user first: this runs the e2e suites on their desktop, parking a nested niri on a spare workspace; keep off it for about two minutes.

```bash
cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh 2>&1 | tee target/container-tests/host.log
grep -E '^(passed:|  (PASS|SKIP|FAIL) |[0-9]+ passed|test result:)' target/container-tests/host.log
```

Expected: five suites `PASS`. Note each suite's inner `passed: N   failed: M   skipped: K` line and `cargo test`'s `test result:` lines — they are what the container must match.

- [ ] **Step 2: e2e-panel.sh — the gate**

Run: `bash tests/container.sh bash tests/e2e-panel.sh; echo "exit: $?"`
Expected: `passed: N   failed: 0   skipped: 0` with N equal to the host's e2e-panel `passed`, and `exit: 0`.

If it fails, the run has already kept its frames and logs in `target/container-tests/<stamp>-artifacts/` (the e2e sandbox under `tmp.*/shots/`, plus `niri.log` and `sway.log`). Look at the failing check's frames with the Read tool, beside the same frames from a host run kept the same way (`NIRITASKS_E2E_KEEP=1 NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`, which prints where it kept them). Tasks 1–2 already pinned fonts, theme, renderer, versions and the nested niri's size, so what is left to differ is mostly speed:
1. `never held still`, `ten frames, no two alike`, a slide that does not finish, or `the daemon exited at once`: software rendering is slower. Check the container sees every CPU (`bash tests/container.sh --no-build nproc` against `nproc`) and nothing else heavy is running. Do not lengthen a test's waits; tell the user.
2. An edge a few pixels off: compare the two frames; with the GTK side shown the same in Task 2, look at what the frame shows that the host's does not.
3. Anything else: stop and ask, with both frames.

**Stop and ask the user** if e2e-panel.sh cannot pass in the container without changing a test or `src/`.

- [ ] **Step 3: The other four, one at a time**

```bash
bash tests/container.sh cargo test; echo "exit: $?"
bash tests/container.sh python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs; echo "exit: $?"
bash tests/container.sh bash tests/e2e-tag.sh; echo "exit: $?"
bash tests/container.sh bash tests/e2e-box.sh; echo "exit: $?"
```

Expected: each `exit: 0`, with the same counts as the host (`test result: ok. N passed` per cargo test binary; `Ran N tests … OK`; e2e-tag `passed: N   failed: 0`, including its six `task start|refine|session` launch checks, which need the refine mod link; e2e-box `passed: N   failed: 0   skipped: 0`). Fix a failure on the container side as in Step 2, and add anything new to the inventory in `check-image.sh` (and `parity.sh` if it is a version the machine has too), so the checks keep describing the image.

- [ ] **Step 4: Commit any fixes**

If Steps 2–3 changed anything: `git add tests/container && git commit` with `fix(container): …` and the cause in the body. If nothing changed, there is nothing to commit.

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
Expected: `parity: the image matches this machine`, `5 passed   0 failed   0 skipped`, every inner summary `skipped: 0`, `exit: 0`. Note the wall time.

While it runs, from a second shell: `docker inspect $(docker ps -q --filter label=niritasks.tests.root="$PWD") --format '{{json .Mounts}} {{.HostConfig.NetworkMode}} {{.HostConfig.Privileged}} {{.HostConfig.CapDrop}}'`
Expected: one mount, `"Type":"volume"` (`niritasks-tests-target-…`), no `"Type":"bind"`; `none`; `false`; `[ALL]`.

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

- [ ] **Step 4: Compare with the host's results, check by check**

Counts can match while different checks pass, so compare the checks themselves, with numbers and parenthesised details (timings, pixel columns, pids) taken out:

```bash
checks() { grep -E '^ *(PASS|FAIL|SKIP) |^test result:|^passed:|^Ran ' "$1" | sed -E 's/\([^)]*\)//g; s/[0-9]+/N/g; s/ +$//' | sort; }
diff <(checks target/container-tests/host.log) <(checks target/container-tests/latest.log) && echo "checks: same"
grep -E '^test result:|^passed:|^Ran ' target/container-tests/host.log > /tmp/h; grep -E '^test result:|^passed:|^Ran ' target/container-tests/latest.log > /tmp/c; diff /tmp/h /tmp/c && echo "counts: same"
```

Expected: `checks: same` and `counts: same`. The one early-returning cargo test (VS Code's — see the inventory) passes on both sides, so it changes neither. Any other difference is a finding: report it with both lines.

- [ ] **Step 5: From another worktree, and two at once**

The main checkout gets these scripts only after the merge, so prove "from any worktree" with a scratch worktree of this branch, and run it beside this one, as two task worktrees would:

```bash
git worktree add ../container-proof HEAD
(cd ../container-proof && bash tests/container.sh bash tests/e2e-panel.sh > /dev/null 2>&1; echo "other: $?") &
bash tests/container.sh bash tests/e2e-panel.sh > /dev/null 2>&1; echo "this: $?"
wait
grep -E 'Compiling|passed:' ../container-proof/target/container-tests/latest.log
git worktree remove --force ../container-proof
bash tests/container.sh --no-build true > /dev/null   # prunes the removed worktree's volume
docker volume ls --filter label=niritasks.tests.root | grep -c container-proof || true
```

Expected: `this: 0` and `other: 0`; the other worktree's log shows only `Compiling niri-tasks` (its new volume came seeded from the same image, no rebuild) and `failed: 0   skipped: 0`; after the removal and one more run, `0` volumes for `container-proof`. If the concurrent pair fails where single runs pass, it is CPU contention on software rendering: tell the user, with the artifacts, rather than serialising runs silently.

- [ ] **Step 6: Document it in README.md**

In "### Running the tests", add under `tests/all.sh` in the code block, with the warm time from Step 2 rounded like its neighbours:

```bash
bash tests/all.sh           # everything this machine can run       ~100s
bash tests/container.sh     # everything, in a Docker container     ~NNNs
```

In the Needs/Touches table, directly under the `tests/all.sh` row:

```markdown
| `tests/container.sh` | Docker, and the user in the `docker` group. The first run builds the image (several minutes, and again after a `Cargo.lock` change) and needs the network; runs do not | Nothing of yours. Docker's own images and volumes (`niritasks-tests*`, pruned as they go stale), and its logs in `target/container-tests/` |
```

After the paragraph ending "…and a long note list scrolls, keeping a row made by Enter at the bottom in view." and before the `> [!IMPORTANT]` box:

````markdown
### Running the tests in a container

`tests/container.sh` runs `tests/all.sh` in a Docker container, against this
checkout as it stands, uncommitted edits included. Nothing of your desktop is
in reach: the container gets no mount, socket or display of yours and no
network, only a copy of the files git would see, so your niri, workspaces,
task database and installed `niritasks` are never touched, and there is no
workspace to keep off. Run it from any worktree, beside other worktrees' runs,
in the foreground or in the background:

```bash
bash tests/container.sh                          # tests/all.sh
bash tests/container.sh bash tests/e2e-box.sh    # one suite
bash tests/container.sh >/dev/null 2>&1 &        # in the background
tail -f target/container-tests/latest.log
```

The image (`tests/container/Dockerfile`) reproduces what the suites use here,
at this machine's versions and in the same folders: niri and ghostty from the
same PPA, Taskwarrior 3.5.0 compiled from its tarball, rustc 1.95.0, herdr,
worktrunk and Claude Code in `~/.local/bin`, Iosevka Term, the GTK theme and
font settings, `en_US.UTF-8`, your timezone and a uid-1000 user. Each run
makes the links `install.sh` makes, from the copy. VS Code and gh are left
out, and Claude Code is never logged in.
`docker run --rm -i "$(bash tests/container/image.sh)" bash -s < tests/container/check-image.sh`
lists what the image holds. Inside, `tests/container/inside.sh` starts a
headless sway, an outer niri in it with two named workspaces, builds the copy
and runs `tests/all.sh` with `NIRITASKS` set to that build; the e2e suites
nest their niri in the outer one as they would in yours.

The image is built on the first run, and again only when the Dockerfile,
`Cargo.toml` or `Cargo.lock` changes (`tests/container/image.sh`). It holds
the crate's dependencies already compiled, and each checkout's build directory
is a Docker volume filled from them, so a checkout's first run compiles only
niri-tasks and later runs only what changed. Each run starts by saying
whether the image still matches this machine — package versions, programs,
font, GTK settings (`tests/container/parity.sh`); when it does not, rebuild
with `NIRITASKS_CONTAINER_REBUILD=1 bash tests/container.sh`. A second run of
the same checkout while one is going refuses, since the two would share a
build directory. Every run is logged in `target/container-tests/`,
`latest.log` the newest; a failed run's logs and the e2e suites' frames are
kept beside it in `<stamp>-artifacts/`. It exits as `tests/all.sh` does.
Volumes of removed worktrees and superseded images are removed by the next
run, and only the three newest images are kept. To reclaim everything:

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

`target/container-tests/` stays uncommitted (it is under `target/`). Report to the user: the host's and the container's summaries side by side, `checks: same` and `counts: same`, the six `same:` lines, the inspect line (one volume, `none`, `[ALL]`), the GTK and parity diffs from Task 2, the concurrent run, and the run time.
