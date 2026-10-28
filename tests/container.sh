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
