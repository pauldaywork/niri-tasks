# Report diagram renderer

`diagrams.mjs` draws a refine report's Mermaid diagrams as SVG before the
page opens, so the page runs no script and works offline. It is
[beautiful-mermaid](https://github.com/lukilabs/beautiful-mermaid) 1.1.3 (MIT)
and its layout engine, elkjs (EPL-2.0), bundled from `entry.ts`.

The mod cannot import it (the mod runtime refuses modules over 1 MiB), so it
runs it as a process, `bun` or else `node`, inside a `bwrap` fence: no
network, no host sockets, no environment, a blank `/tmp`, and only `/usr`,
the runtime and the script visible, read-only (`hooks/draw.ts`).

The bundle was built with bun 1.3.13; build with the same version for a
byte-identical `diagrams.mjs`. Rebuild after changing `entry.ts` or the
version in `package.json`:

    bun install && bun run build && rm -rf node_modules
