# The task panel is drawn by the daemon in GTK, not in Quickshell

The task panel is a gtk4-layer-shell surface owned by `niritasks daemon`, and
it maintains its own click-through input region by hand
(`gdk::Surface::set_input_region`, reset whenever the cards move). Quickshell
would have been less code for the surface itself: it was already installed, and
its `mask: Region { item: … }` tracks a sliding item declaratively.

It lost because the panel is mostly *not* the surface. What it shows is the
workspace tag rule, the pending-task query and the change detection the daemon
already has, and a Quickshell panel would have had to reach those by shelling
out to `niritasks` and `task export`, or by carrying a second copy of the tag
rule. Two definitions of "this workspace's tasks" is the drift this repo exists
to prevent. It would also have brought back a runtime that was removed with DMS,
and needed its own launcher and unit beside the one this daemon already has.

## Considered options

- **eww, waybar, fabric, AGS/Astal**: none can make part of a window
  click-through. The transparent area around the cards would swallow clicks
  meant for the windows beneath, so they are out whatever else they offer.
- **Quickshell**: technically the best fit, rejected for the reasons above.

## Consequences

A spike on niri 26.04 confirmed that niri honours a layer surface's input
region: clicks beside the peek reach the window beneath, and pointer enter/leave
follows the region as it grows and shrinks. If a future niri stops doing that,
Quickshell would break in the same way, since it relies on the same protocol.
