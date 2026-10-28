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
# A rebuild leaves the image it replaces untagged; that one is removed after
# the build (unless a container still uses it).
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
    old=$(docker image inspect -f '{{.Id}}' "$IMAGE" 2>/dev/null || true)
    echo "building $IMAGE: once, and again only when one of ${INPUTS[*]} changes" >&2
    tar -cf - "${INPUTS[@]}" |
        docker build "${fresh[@]}" -t "$IMAGE" -f tests/container/Dockerfile - >&2
    new=$(docker image inspect -f '{{.Id}}' "$IMAGE" 2>/dev/null || true)
    if [ -n "$old" ] && [ "$old" != "$new" ]; then
        docker image rm "$old" >/dev/null 2>&1 || true
    fi
    # Newest first, as docker lists them; past the third image, every tag goes.
    docker image ls niritasks-tests --format '{{.ID}} {{.Repository}}:{{.Tag}}' |
        awk '!($1 in seen) { seen[$1]; n++ } n > 3 { print $2 }' |
        xargs -r docker image rm >/dev/null 2>&1 || true
fi
echo "$IMAGE"
