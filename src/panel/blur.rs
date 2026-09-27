//! Blur behind the task cards, and only behind them.
//!
//! A niri `layer-rule` blurs a layer surface's whole rectangle, and the
//! panel's surface is mostly empty space the cards slide through — a rule
//! would draw a blurred box around them. So the panel asks for blur itself,
//! over `ext-background-effect-v1`, and hands niri a region the shape of the
//! cards. niri turns blur on for a surface that sets one, with xray, and takes
//! noise and saturation from the layer-rule in `niri/niri-tasks.kdl`.
//!
//! The region is rectangles, and niri does not round it off to a corner radius
//! for layer surfaces, so each card's rounded corners are built out of 1px
//! strips. Square corners would let the blur show past the card's own.
//!
//! GTK owns the Wayland connection; this borrows it on a queue of its own.
//! A compositor without the protocol means no blur, not an error.

use gdk4_wayland::prelude::*;
use gtk4::prelude::*;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_compositor, wl_region, wl_registry};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1 as manager, ext_background_effect_surface_v1 as effect,
};

/// A rectangle in surface coordinates: x, y, width, height.
pub type Rect = (i32, i32, i32, i32);

pub struct Blur {
    conn: Connection,
    queue: EventQueue<Handler>,
    compositor: wl_compositor::WlCompositor,
    effect: effect::ExtBackgroundEffectSurfaceV1,
    last: Vec<Rect>,
}

impl Blur {
    /// Ask for blur on this window's surface. `None` when the window is not
    /// mapped on Wayland yet, or the compositor cannot blur.
    pub fn new(window: &gtk4::ApplicationWindow) -> Option<Blur> {
        let surface = window
            .surface()?
            .downcast::<gdk4_wayland::WaylandSurface>()
            .ok()?;
        let display = surface
            .display()
            .downcast::<gdk4_wayland::WaylandDisplay>()
            .ok()?;
        let wl_surface = surface.wl_surface()?;
        let wl_display = display.wl_display()?;
        let conn = Connection::from_backend(wayland_client::Proxy::backend(&wl_display).upgrade()?);

        let (globals, queue) = registry_queue_init::<Handler>(&conn).ok()?;
        let qh = queue.handle();
        let compositor = globals
            .bind::<wl_compositor::WlCompositor, _, _>(&qh, 1..=6, ())
            .ok()?;
        let manager = globals
            .bind::<manager::ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
            .ok()?;
        let effect = manager.get_background_effect(&wl_surface, &qh, ());

        Some(Blur {
            conn,
            queue,
            compositor,
            effect,
            last: Vec::new(),
        })
    }

    /// Blur behind exactly these rectangles, from the surface's next commit.
    pub fn set(&mut self, rects: &[Rect]) {
        if self.last == rects {
            return;
        }
        self.last = rects.to_vec();

        let qh = self.queue.handle();
        let region = self.compositor.create_region(&qh, ());
        for &(x, y, w, h) in rects {
            region.add(x, y, w, h);
        }
        self.effect.set_blur_region(Some(&region));
        region.destroy();
        let _ = self.conn.flush();
        // Nothing here needs the compositor's replies, but they still arrive
        // on this queue; drain them so they do not pile up.
        let _ = self.queue.dispatch_pending(&mut Handler);
    }
}

impl Drop for Blur {
    fn drop(&mut self) {
        self.effect.destroy();
        let _ = self.conn.flush();
    }
}

/// A card's area as rectangles, with its corners rounded off.
///
/// `round_right` is false while the card runs off the screen edge: its right
/// corners are not on screen, and the region stops at the edge square.
pub fn card_region((x, y, w, h): Rect, radius: i32, round_right: bool) -> Vec<Rect> {
    let r = radius.min(w / 2).min(h / 2).max(0);
    if r == 0 {
        return vec![(x, y, w, h)];
    }
    let mut rects = Vec::with_capacity(2 * r as usize + 1);
    for i in 0..r {
        // How far the corner's arc is in from the side, at this row's middle.
        let dy = (r - i) as f64 - 0.5;
        let inset = r - ((r * r) as f64 - dy * dy).max(0.0).sqrt().round() as i32;
        let right = if round_right { inset } else { 0 };
        let width = w - inset - right;
        rects.push((x + inset, y + i, width, 1));
        rects.push((x + inset, y + h - 1 - i, width, 1));
    }
    rects.push((x, y + r, w, h - 2 * r));
    rects
}

pub struct Handler;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Handler {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_compositor::WlCompositor, ()> for Handler {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_region::WlRegion, ()> for Handler {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<manager::ExtBackgroundEffectManagerV1, ()> for Handler {
    fn event(
        _: &mut Self,
        _: &manager::ExtBackgroundEffectManagerV1,
        _: manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<effect::ExtBackgroundEffectSurfaceV1, ()> for Handler {
    fn event(
        _: &mut Self,
        _: &effect::ExtBackgroundEffectSurfaceV1,
        _: effect::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(rects: &[Rect]) -> i32 {
        rects.iter().map(|&(_, _, w, h)| w * h).sum()
    }

    #[test]
    fn no_radius_is_the_plain_rectangle() {
        assert_eq!(card_region((1, 2, 30, 40), 0, true), vec![(1, 2, 30, 40)]);
    }

    #[test]
    fn rounding_trims_the_corners_and_nothing_else() {
        let rects = card_region((0, 0, 380, 46), 8, true);
        let full = 380 * 46;
        let trimmed = full - area(&rects);
        // Four corners of an 8px radius: each 8x8 square less a quarter
        // circle is about 13.7px, so ~55 in all.
        assert!((45..=65).contains(&trimmed), "trimmed {trimmed}px");
        // Every strip stays inside the card.
        for &(x, y, w, h) in &rects {
            assert!(x >= 0 && y >= 0 && x + w <= 380 && y + h <= 46);
        }
    }

    #[test]
    fn the_top_row_is_inset_most_and_the_middle_not_at_all() {
        let rects = card_region((0, 0, 100, 40), 8, true);
        assert!(rects[0].0 >= 4, "top row is barely inset: {:?}", rects[0]);
        assert_eq!(*rects.last().unwrap(), (0, 8, 100, 24));
    }

    #[test]
    fn a_card_off_the_edge_keeps_its_right_side_square() {
        let rects = card_region((0, 0, 100, 40), 8, false);
        for &(x, _, w, _) in &rects {
            assert_eq!(x + w, 100, "a strip stops short of the edge");
        }
    }

    #[test]
    fn a_radius_bigger_than_the_card_is_clamped() {
        let rects = card_region((0, 0, 10, 6), 8, true);
        for &(_, _, w, h) in &rects {
            assert!(w >= 0 && h >= 0);
        }
    }
}
