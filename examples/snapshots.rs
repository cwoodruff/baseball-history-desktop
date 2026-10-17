//! Renders app screens offscreen to PNG files, for visual review without a
//! display or screen-recording permission.
//!
//! ```sh
//! cargo run --example snapshots -- <out-dir> [dark] [route ...]
//! ```
//!
//! Routes: home, players, player:<id>, teams, franchise:<id>,
//! season:<team>:<lg>:<year>, parks, park:<key>, batting, pitching, hof,
//! awards, postseason, salaries, search:<q>, compare:<id>:<id>, about,
//! glossary, surviving, and type:<q> to type into the global search box.
//! Data is fetched from the live API.

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn main() {
    use std::sync::Arc;
    use std::time::Duration;

    use baseball_history_desktop::route::{ReferencePage, Route};
    use baseball_history_desktop::{theme, workspace::Workspace};
    use gpui_kit::component::ThemeMode;
    use gpui_kit::{AppContext as _, HeadlessAppContext, px, size};

    let mut args = std::env::args().skip(1);
    let out = args.next().expect("usage: snapshots <out-dir> [dark] [route ...]");
    std::fs::create_dir_all(&out).unwrap();
    let mut rest: Vec<String> = args.collect();
    let dark = rest.first().is_some_and(|a| a == "dark");
    if dark {
        rest.remove(0);
    }
    if rest.is_empty() {
        rest = vec!["home".into()];
    }

    let parse = |spec: &str| -> Route {
        let parts: Vec<&str> = spec.split(':').collect();
        match parts.as_slice() {
            ["home"] => Route::Home,
            ["players"] => Route::Players { letter: 'A' },
            ["players", l] => Route::Players { letter: l.chars().next().unwrap() },
            ["player", id] => Route::Player { id: id.to_string() },
            ["teams"] => Route::Teams,
            ["franchise", id] => Route::Franchise { id: id.to_string() },
            ["season", t, l, y] => Route::TeamSeason {
                team_id: t.to_string(),
                lg_id: l.to_string(),
                year: y.parse().unwrap(),
            },
            ["parks"] => Route::Parks,
            ["park", key] => Route::Park { key: key.to_string() },
            ["batting"] => Route::BattingLeaders,
            ["pitching"] => Route::PitchingLeaders,
            ["hof"] => Route::HallOfFame { year: None, category: None },
            ["awards"] => Route::Awards,
            ["postseason"] => Route::Postseason { year: None },
            ["salaries"] => Route::Salaries,
            ["search", q] => Route::Search { query: q.to_string() },
            ["compare"] => Route::Compare { first: None, second: None },
            ["compare", a, b] => Route::Compare {
                first: Some(a.to_string()),
                second: Some(b.to_string()),
            },
            ["about"] => Route::Reference(ReferencePage::About),
            ["glossary"] => Route::Reference(ReferencePage::Glossary),
            ["surviving"] => Route::Reference(ReferencePage::SurvivingRecords),
            other => panic!("unknown route {other:?}"),
        }
    };

    let mut cx = HeadlessAppContext::with_platform(
        gpui_kit::platform::current_platform(true).text_system(),
        Arc::new(baseball_history_desktop::assets::AppAssets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::apply(if dark { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
        baseball_history_desktop::workspace::bind_keys(cx);
    });

    let mut workspace = None;
    let (handle, _) = cx
        .update(|cx| {
            gpui_kit::open_window(
                gpui_kit::WindowOptions {
                    window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                        origin: Default::default(),
                        size: size(px(1320.), px(860.)),
                    })),
                    focus: false,
                    show: false,
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    let view = cx.new(|cx| Workspace::new(window, cx));
                    workspace = Some(view.clone());
                    view
                },
            )
        })
        .unwrap();
    let workspace = workspace.unwrap();

    let settle = |cx: &mut HeadlessAppContext| {
        for _ in 0..8 {
            cx.run_until_parked();
            cx.advance_clock(Duration::from_millis(400));
            cx.update_window(handle, |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
        }
    };

    for (n, spec) in rest.iter().enumerate() {
        if let Some(query) = spec.strip_prefix("type:") {
            // Types into the global search box to capture its dropdown.
            cx.update_window(handle, |_, window, cx| {
                workspace.update(cx, |workspace, cx| workspace.type_search(query, window, cx));
            })
            .unwrap();
        } else {
            let route = parse(spec);
            cx.update_window(handle, |_, window, cx| {
                workspace.update(cx, |workspace, cx| workspace.navigate(route, window, cx));
            })
            .unwrap();
        }
        settle(&mut cx);
        let image = cx.capture_screenshot(handle).expect("offscreen rendering");
        let name = spec.replace(':', "_");
        let path = format!("{out}/{n:02}-{name}{}.png", if dark { "-dark" } else { "" });
        image.save(&path).unwrap();
        println!("{path}");
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {
    println!("snapshots: GPUI has no headless renderer on this platform");
}
