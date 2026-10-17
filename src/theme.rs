//! Theme setup: GPUI Kit's light and dark themes with the web app's brand colors.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, Window, rgb};

/// Switches to `mode` and re-applies the brand palette over it.
pub fn apply(mode: ThemeMode, window: Option<&mut Window>, cx: &mut App) {
    Theme::change(mode, window, cx);
    // Colors must be edited in a second update: changing mode reloads them.
    Theme::update(cx, |theme| {
        let dark = theme.mode.is_dark();
        // Brand navy (#002D72) is too dark to read as an accent on a dark
        // background, so dark mode uses the web app's dark-theme link blue.
        let primary: Hsla = if dark { rgb(0x3d6fc4).into() } else { rgb(0x002d72).into() };
        let hover: Hsla = if dark { rgb(0x5583d1).into() } else { rgb(0x1a4a8f).into() };
        let active: Hsla = if dark { rgb(0x2f5ca8).into() } else { rgb(0x001a4d).into() };
        let colors = &mut theme.colors;
        colors.primary = primary;
        colors.primary_hover = hover;
        colors.primary_active = active;
        colors.primary_foreground = rgb(0xffffff).into();
        colors.button_primary = primary;
        colors.button_primary_hover = hover;
        colors.button_primary_active = active;
        colors.button_primary_foreground = rgb(0xffffff).into();
        colors.ring = primary;
        colors.link = if dark { rgb(0x8fb3ee).into() } else { rgb(0x002d72).into() };
        colors.link_hover = if dark { rgb(0xb3ccf5).into() } else { rgb(0x1a4a8f).into() };
        colors.sidebar_primary = primary;
        colors.sidebar_primary_foreground = rgb(0xffffff).into();
    });
}
