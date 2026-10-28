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
RUN mkdir -p /etc/dconf/profile \
 && printf 'user-db:user\nsystem-db:local\n' > /etc/dconf/profile/user \
 && dconf update

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
