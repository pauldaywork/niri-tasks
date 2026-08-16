//! Colours, read from the DankMaterialShell theme when one is present.
//!
//! The QML task box this replaces bound 40 `Theme.*` properties and so followed
//! the shell's theme for free. A separate process cannot do that, but DMS
//! stores its palette as plain JSON with the same token names, so the box reads
//! that file and matches.
//!
//! DMS is optional, which is the point of the whole redesign — so every lookup
//! falls back to a built-in palette and a missing or malformed theme file is
//! never an error.

use serde::Deserialize;
use std::path::PathBuf;

/// The subset of DMS's palette the box actually draws with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub surface_container: String,
    pub surface_text: String,
    pub surface_text_medium: String,
    pub surface_variant: String,
    pub primary: String,
    pub error: String,
}

impl Default for Theme {
    /// The built-in palette, used when DMS is not installed. These are the
    /// values from the "Peace & Quiet" dark variant, so a machine without DMS
    /// still looks deliberate rather than like a default GTK dialog.
    fn default() -> Self {
        Self {
            surface_container: "#09070d".into(),
            surface_text: "#f0f0f0".into(),
            surface_text_medium: "#b0b0b0".into(),
            surface_variant: "#221d26".into(),
            primary: "#49ccd2".into(),
            error: "#f38ba8".into(),
        }
    }
}

#[derive(Deserialize)]
struct ThemeFile {
    #[serde(default)]
    dark: Option<Variant>,
    #[serde(default)]
    light: Option<Variant>,
}

#[derive(Deserialize, Default)]
struct Variant {
    #[serde(rename = "surfaceContainer")]
    surface_container: Option<String>,
    #[serde(rename = "surfaceText")]
    surface_text: Option<String>,
    #[serde(rename = "surfaceVariant")]
    surface_variant: Option<String>,
    #[serde(rename = "primary")]
    primary: Option<String>,
    #[serde(rename = "error")]
    error: Option<String>,
}

impl Theme {
    /// Load the active DMS theme, falling back to the built-in palette.
    pub fn load() -> Self {
        Self::from_dir(config_dir().map(|c| c.join("DankMaterialShell")))
    }

    fn from_dir(dms_dir: Option<PathBuf>) -> Self {
        let Some(dir) = dms_dir else {
            return Self::default();
        };

        // DMS records the chosen theme in settings.json; rather than parse that
        // too, take whatever single theme is installed under themes/. There is
        // one in practice, and guessing wrong only costs colours.
        let Ok(entries) = std::fs::read_dir(dir.join("themes")) else {
            return Self::default();
        };

        for entry in entries.flatten() {
            let path = entry.path().join("theme.json");
            if let Some(theme) = Self::from_file(&path) {
                return theme;
            }
        }
        Self::default()
    }

    fn from_file(path: &std::path::Path) -> Option<Self> {
        let raw = std::fs::read_to_string(path).ok()?;
        Self::from_json(&raw)
    }

    /// Parse a DMS theme.json. Prefers the dark variant.
    pub fn from_json(raw: &str) -> Option<Self> {
        let parsed: ThemeFile = serde_json::from_str(raw).ok()?;
        let v = parsed.dark.or(parsed.light)?;

        let d = Self::default();
        Some(Self {
            surface_text_medium: v
                .surface_text
                .clone()
                .map(|c| dim(&c))
                .unwrap_or(d.surface_text_medium),
            surface_container: v.surface_container.unwrap_or(d.surface_container),
            surface_text: v.surface_text.unwrap_or(d.surface_text),
            surface_variant: v.surface_variant.unwrap_or(d.surface_variant),
            primary: v.primary.unwrap_or(d.primary),
            error: v.error.unwrap_or(d.error),
        })
    }

    /// The box's stylesheet.
    pub fn css(&self) -> String {
        format!(
            "
window {{ background-color: {bg}; }}
label {{ color: {fg}; }}
label.dim {{ color: {dim}; font-size: 90%; }}
textview {{ background-color: {variant}; color: {fg}; padding: 8px; border-radius: 8px; }}
textview text {{ background-color: transparent; color: {fg}; }}
scrolledwindow.notes {{ background-color: {variant}; border-radius: 8px; }}
button {{ background-image: none; background-color: {variant}; color: {fg};
          border: 0; border-radius: 8px; padding: 6px 16px; }}
button.suggested {{ background-color: {primary}; color: {bg}; font-weight: bold; }}
",
            bg = self.surface_container,
            fg = self.surface_text,
            dim = self.surface_text_medium,
            variant = self.surface_variant,
            primary = self.primary,
        )
    }
}

/// Render a `#rrggbb` token as a GTK `rgba()` with an alpha channel.
///
/// Needed because the palette is opaque hex and the overlay wants to be seen
/// through. Anything unparseable is passed back untouched rather than becoming
/// black — a wrong colour is survivable, an invisible readout is not.
pub fn with_alpha(hex: &str, alpha: f32) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 || !h.as_bytes()[..6].iter().all(u8::is_ascii_hexdigit) {
        return hex.to_string();
    }
    let ch = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    format!("rgba({}, {}, {}, {alpha})", ch(0), ch(2), ch(4))
}

/// DMS's `surfaceTextMedium` is a dimmed `surfaceText`; approximate it by
/// pulling each channel toward the middle rather than shipping a second token.
fn dim(hex: &str) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 {
        return hex.to_string();
    }
    let parse = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0x80);
    let (r, g, b) = (parse(0), parse(2), parse(4));
    let blend = |c: u8| ((c as u16 * 7 + 0x80 * 3) / 10) as u8;
    format!("#{:02x}{:02x}{:02x}", blend(r), blend(g), blend(b))
}

fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real shape of ~/.config/DankMaterialShell/themes/*/theme.json.
    // r##"…"## because the JSON contains `"#` (a quoted hex colour), which
    // would close an r#"…"# literal early.
    const REAL: &str = r##"{
        "id": "peaceAndQuiet",
        "name": "Peace & Quiet",
        "dark": {
            "surface": "#130f1a",
            "surfaceText": "#f0f0f0",
            "surfaceVariant": "#221d26",
            "surfaceContainer": "#09070d",
            "error": "#f38ba8"
        },
        "light": {
            "surfaceText": "#1e1824",
            "surfaceContainer": "#f5f0fa"
        }
    }"##;

    #[test]
    fn reads_the_dark_variant_from_a_real_theme_file() {
        let t = Theme::from_json(REAL).expect("parses");
        assert_eq!(t.surface_container, "#09070d");
        assert_eq!(t.surface_text, "#f0f0f0");
        assert_eq!(t.surface_variant, "#221d26");
        assert_eq!(t.error, "#f38ba8");
    }

    /// Tokens the file omits fall back rather than rendering as empty CSS.
    #[test]
    fn missing_tokens_fall_back_to_the_builtin_palette() {
        let t = Theme::from_json(REAL).expect("parses");
        assert_eq!(t.primary, Theme::default().primary);
    }

    #[test]
    fn malformed_json_is_not_an_error() {
        assert!(Theme::from_json("not json").is_none());
        assert!(Theme::from_json("{}").is_none());
    }

    /// A machine with no DMS at all still gets a full palette.
    #[test]
    fn absent_dms_yields_the_builtin_palette() {
        assert_eq!(Theme::from_dir(None), Theme::default());
        assert_eq!(
            Theme::from_dir(Some(PathBuf::from("/nonexistent"))),
            Theme::default()
        );
    }

    #[test]
    fn with_alpha_renders_gtk_rgba() {
        assert_eq!(with_alpha("#09070d", 0.72), "rgba(9, 7, 13, 0.72)");
        // The leading # is optional, as it is for dim().
        assert_eq!(with_alpha("ffffff", 1.0), "rgba(255, 255, 255, 1)");
    }

    /// A malformed token must pass through, not silently become black — the
    /// overlay would be a black bar rather than an obviously wrong colour.
    #[test]
    fn with_alpha_passes_through_anything_unparseable() {
        assert_eq!(with_alpha("bad", 0.5), "bad");
        assert_eq!(with_alpha("#gggggg", 0.5), "#gggggg");
    }

    #[test]
    fn dim_pulls_toward_the_middle() {
        // 70% of the channel plus 30% of mid-grey: 255 -> 216, 0 -> 38.
        assert_eq!(dim("#ffffff"), "#d8d8d8");
        assert_eq!(dim("#000000"), "#262626");
        // Both ends move inward, so a dimmed colour is never more extreme.
        assert_eq!(dim("#808080"), "#808080", "mid-grey is the fixed point");
        // Anything unparseable is passed through rather than becoming black.
        assert_eq!(dim("bad"), "bad");
    }

    /// Every colour must reach the stylesheet — an unsubstituted token would
    /// render as a GTK parse error and a default-looking box.
    #[test]
    fn css_contains_every_colour() {
        let t = Theme::from_json(REAL).expect("parses");
        let css = t.css();
        for c in [&t.surface_container, &t.surface_text, &t.surface_variant, &t.primary] {
            assert!(css.contains(c.as_str()), "css missing {c}");
        }
        // CSS is full of literal braces, so the meaningful check is that no
        // format placeholder survived unsubstituted — that would render as a
        // GTK parse error and a default-looking box.
        for placeholder in ["{bg}", "{fg}", "{dim}", "{variant}", "{primary}"] {
            assert!(!css.contains(placeholder), "css left {placeholder} unsubstituted");
        }
    }
}
