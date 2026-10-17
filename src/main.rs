//! Baseball History: a desktop client for the Baseball History Project API.

use baseball_history_desktop::{theme, workspace::{self, Workspace}};
use gpui_kit::component::{ThemeMode, TitleBar};
use gpui_kit::*;

actions!(app, [Quit]);

fn main() {
    gpui_kit::application()
        .with_assets(baseball_history_desktop::assets::AppAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            let mode = match cx.window_appearance() {
                WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
                _ => ThemeMode::Light,
            };
            theme::apply(mode, None, cx);
            workspace::bind_keys(cx);
            cx.bind_keys([KeyBinding::new("secondary-q", Quit, None)]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.set_menus(vec![Menu {
                name: "Baseball History".into(),
                items: vec![MenuItem::action("Quit Baseball History", Quit)],
                disabled: false,
            }]);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1320.), px(860.)), cx)),
                window_min_size: Some(size(px(900.), px(600.))),
                titlebar: Some(TitleBar::title_bar_options()),
                app_id: Some("com.baseballhistoryproject.desktop".into()),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                window.set_window_title("Baseball History");
                let workspace = cx.new(|cx| Workspace::new(window, cx));
                workspace.read(cx).focus_handle(cx).focus(window, cx);
                workspace
            })
            .expect("failed to open the main window");
            cx.activate(true);
        });
}
