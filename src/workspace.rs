//! The application shell: title bar, sidebar, history and global search.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _, ThemeMode,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    sidebar::{Sidebar, SidebarCollapsible, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    v_flex, TitleBar,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gpui_kit::assets::IconName;

use crate::api::models::SearchResults;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::{self, PageView};
use crate::route::{ReferencePage, Route, SIDEBAR};
use crate::ui::widgets;

actions!(
    baseball,
    [Back, Forward, Reload, FocusSearch, ToggleTheme, ToggleSidebar, Home, CloseSearch]
);

const CONTEXT: &str = "Workspace";
const MAX_HISTORY: usize = 50;

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-[", Back, Some(CONTEXT)),
        KeyBinding::new("secondary-]", Forward, Some(CONTEXT)),
        KeyBinding::new("alt-left", Back, Some(CONTEXT)),
        KeyBinding::new("alt-right", Forward, Some(CONTEXT)),
        KeyBinding::new("secondary-r", Reload, Some(CONTEXT)),
        KeyBinding::new("f5", Reload, Some(CONTEXT)),
        KeyBinding::new("secondary-f", FocusSearch, Some(CONTEXT)),
        KeyBinding::new("secondary-k", FocusSearch, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-l", ToggleTheme, Some(CONTEXT)),
        KeyBinding::new("secondary-b", ToggleSidebar, Some(CONTEXT)),
        KeyBinding::new("secondary-shift-h", Home, Some(CONTEXT)),
        KeyBinding::new("escape", CloseSearch, Some(CONTEXT)),
    ]);
}

struct Entry {
    route: Route,
    page: PageView,
}

pub struct Workspace {
    history: Vec<Entry>,
    cursor: usize,
    nav: Navigator,
    focus_handle: FocusHandle,
    search: Entity<InputState>,
    search_results: Loadable<SearchResults>,
    search_open: bool,
    search_task: Option<Task<()>>,
    sidebar_collapsed: bool,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let nav = Navigator::new(cx.entity().downgrade());
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search players, teams…"));
        let subscription = cx.subscribe_in(&search, window, Self::on_search_event);
        let home = pages::build(&Route::Home, &nav, window, cx);
        Self {
            history: vec![Entry { route: Route::Home, page: home }],
            cursor: 0,
            nav,
            focus_handle: cx.focus_handle(),
            search,
            search_results: Loadable::Idle,
            search_open: false,
            search_task: None,
            sidebar_collapsed: false,
            _subscriptions: vec![subscription],
        }
    }

    fn current(&self) -> &Entry {
        &self.history[self.cursor]
    }

    pub fn navigate(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        self.close_search(window, cx);
        if self.current().route == route {
            return;
        }
        let page = pages::build(&route, &self.nav, window, cx);
        self.history.truncate(self.cursor + 1);
        self.history.push(Entry { route, page });
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
        self.cursor = self.history.len() - 1;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn can_go_back(&self) -> bool {
        self.cursor > 0
    }

    fn can_go_forward(&self) -> bool {
        self.cursor + 1 < self.history.len()
    }

    fn back(&mut self, _: &Back, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_go_back() {
            self.cursor -= 1;
            self.close_search(window, cx);
            cx.notify();
        }
    }

    fn forward(&mut self, _: &Forward, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_go_forward() {
            self.cursor += 1;
            self.close_search(window, cx);
            cx.notify();
        }
    }

    fn reload(&mut self, _: &Reload, window: &mut Window, cx: &mut Context<Self>) {
        self.current().page.reload(window, cx);
    }

    fn go_home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate(Route::Home, window, cx);
    }

    fn focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |state, cx| state.focus(window, cx));
    }

    fn on_close_search(&mut self, _: &CloseSearch, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            self.close_search(window, cx);
        } else {
            cx.propagate();
        }
    }

    fn toggle_theme(&mut self, _: &ToggleTheme, window: &mut Window, cx: &mut Context<Self>) {
        let mode = if cx.theme().is_dark() { ThemeMode::Light } else { ThemeMode::Dark };
        crate::theme::apply(mode, Some(window), cx);
    }

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    // MARK: Global search

    fn on_search_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let query = state.read(cx).value().trim().to_string();
                self.query_changed(query, cx);
            }
            InputEvent::PressEnter { .. } => {
                let query = state.read(cx).value().trim().to_string();
                if !query.is_empty() {
                    self.navigate(Route::Search { query }, window, cx);
                }
            }
            InputEvent::Focus => {
                if !matches!(self.search_results, Loadable::Idle) {
                    self.search_open = true;
                    cx.notify();
                }
            }
            InputEvent::Blur => {}
        }
    }

    /// Debounces the global search like the web header (300 ms), then queries.
    fn query_changed(&mut self, query: String, cx: &mut Context<Self>) {
        if query.chars().count() < 2 {
            self.search_task = None;
            self.search_results = Loadable::Idle;
            self.search_open = false;
            cx.notify();
            return;
        }
        self.search_open = true;
        self.search_results = Loadable::Loading;
        self.search_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(300)).await;
            let Ok(task) = this.update(cx, |_, cx| {
                loader::fetch(
                    cx,
                    move |api| api.search(&query, 10),
                    |this: &mut Workspace, result, _| {
                        this.search_results = Loadable::from_result(result);
                    },
                )
            }) else {
                return;
            };
            task.await;
        }));
        cx.notify();
    }

    /// Types `query` into the global search box, as if the user had.
    pub fn type_search(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |state, cx| state.set_value(query.to_string(), window, cx));
        self.query_changed(query.trim().to_string(), cx);
    }

    fn close_search(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            self.search_open = false;
            cx.notify();
        }
    }

    fn render_search_results(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.search.read(cx).value().trim().to_string();
        let body: AnyElement = match &self.search_results {
            Loadable::Loading => widgets::loading_state("Searching…", cx).into_any_element(),
            Loadable::Failed(message) => div()
                .p_3()
                .text_sm()
                .text_color(cx.theme().danger)
                .child(message.clone())
                .into_any_element(),
            Loadable::Ready(results) if results.players.is_empty() && results.franchises.is_empty() => {
                v_flex()
                    .p_3()
                    .gap_2()
                    .child(div().text_sm().font_semibold().child(format!("No results found for “{query}”")))
                    .child(widgets::note("Check the spelling or try just a last name.", cx))
                    .child(div().text_xs().font_semibold().text_color(cx.theme().muted_foreground).child("POPULAR PLAYERS"))
                    .child(
                        h_flex().gap_1().flex_wrap().children(
                            [
                                ("ruthba01", "Babe Ruth"),
                                ("aaronha01", "Hank Aaron"),
                                ("mayswi01", "Willie Mays"),
                                ("jeterde01", "Derek Jeter"),
                                ("ohtansh01", "Shohei Ohtani"),
                            ]
                            .into_iter()
                            .map(|(id, name)| {
                                Button::new(SharedString::from(format!("popular-{id}")))
                                    .xsmall()
                                    .outline()
                                    .label(name)
                                    .on_click(self.nav.to(Route::Player { id: id.into() }))
                            }),
                        ),
                    )
                    .child(
                        h_flex().child(
                            Button::new("browse-all-players")
                                .xsmall()
                                .ghost()
                                .label("Browse all players")
                                .on_click(self.nav.to(Route::Players { letter: 'A' })),
                        ),
                    )
                    .into_any_element()
            }
            Loadable::Ready(results) => {
                let mut list = v_flex().py_1();
                if !results.players.is_empty() {
                    list = list.child(group_label("PLAYERS", cx));
                    for player in results.players.iter().take(10) {
                        let name = player.full_name.clone().unwrap_or_else(|| player.player_id.clone());
                        list = list.child(search_row(
                            SharedString::from(format!("search-player-{}", player.player_id)),
                            IconName::User,
                            name,
                            format::year_span(player.debut_year, player.final_year),
                            player.is_hall_of_famer,
                            self.nav.to(Route::Player { id: player.player_id.clone() }),
                            cx,
                        ));
                    }
                }
                if !results.franchises.is_empty() {
                    list = list.child(group_label("TEAMS", cx));
                    for team in results.franchises.iter().take(5) {
                        let name = team.franchise_name.clone().unwrap_or_else(|| team.franchise_id.clone());
                        let subtitle = if team.is_active { "Active Franchise" } else { "Historical Franchise" };
                        list = list.child(search_row(
                            SharedString::from(format!("search-team-{}", team.franchise_id)),
                            IconName::Flag,
                            name,
                            subtitle.to_string(),
                            false,
                            self.nav.to(Route::Franchise { id: team.franchise_id.clone() }),
                            cx,
                        ));
                    }
                }
                let more = results.total_player_count as usize > results.players.len()
                    || results.total_franchise_count as usize > results.franchises.len();
                list = list.child(
                    h_flex().px_2().pt_1().border_t_1().border_color(cx.theme().border).child(
                        Button::new("search-see-all")
                            .xsmall()
                            .ghost()
                            .label(if more {
                                format!("See all {} results", results.total_player_count + results.total_franchise_count)
                            } else {
                                "Open search page".to_string()
                            })
                            .on_click(self.nav.to(Route::Search { query: query.clone() })),
                    ),
                );
                list.into_any_element()
            }
            Loadable::Idle => div().into_any_element(),
        };

        deferred(
            // Right-aligned under the 300px search box, opening downward.
            anchored()
                .position_mode(AnchoredPositionMode::Local)
                .anchor(Anchor::TopRight)
                .position(point(px(300.), px(30.)))
                .snap_to_window_with_margin(px(8.))
                .child(
                div()
                    .id("search-results")
                    .occlude()
                    .mt_1()
                    .w(px(380.))
                    .max_h(px(480.))
                    .overflow_y_scroll()
                    .bg(cx.theme().popover)
                    .text_color(cx.theme().popover_foreground)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius_lg)
                    .shadow_lg()
                    .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close_search(window, cx)))
                    .child(body),
            ),
        )
        .with_priority(1)
        .into_any_element()
    }

    // MARK: Chrome

    fn render_title_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let title = self.current().page.title(cx);
        let is_dark = cx.theme().is_dark();
        TitleBar::new().child(
            h_flex()
                .w_full()
                .pr_2()
                .gap_2()
                .items_center()
                .justify_between()
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .min_w_0()
                        .child(
                            Button::new("nav-back")
                                .ghost()
                                .small()
                                .icon(IconName::ChevronLeft)
                                .accessibility_label("Back")
                                .tooltip_with_action("Back", &Back, Some(CONTEXT))
                                .disabled(!self.can_go_back())
                                .on_click(cx.listener(|this, _, window, cx| this.back(&Back, window, cx))),
                        )
                        .child(
                            Button::new("nav-forward")
                                .ghost()
                                .small()
                                .icon(IconName::ChevronRight)
                                .accessibility_label("Forward")
                                .tooltip_with_action("Forward", &Forward, Some(CONTEXT))
                                .disabled(!self.can_go_forward())
                                .on_click(cx.listener(|this, _, window, cx| this.forward(&Forward, window, cx))),
                        )
                        .child(
                            div()
                                .ml_1()
                                .text_sm()
                                .font_semibold()
                                .truncate()
                                .child(title),
                        ),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(
                            v_flex()
                                .w(px(300.))
                                .child(
                                    Input::new(&self.search)
                                        .small()
                                        .cleanable(true)
                                        .prefix(gpui_kit::component::Icon::new(IconName::Search).small())
                                        .aria_label("Search players and teams"),
                                )
                                .when(self.search_open, |this| this.child(self.render_search_results(cx))),
                        )
                        .child(
                            Button::new("refresh")
                                .ghost()
                                .small()
                                .icon(IconName::RefreshCw)
                                .accessibility_label("Reload")
                                .tooltip_with_action("Reload", &Reload, Some(CONTEXT))
                                .on_click(cx.listener(|this, _, window, cx| this.reload(&Reload, window, cx))),
                        )
                        .child(
                            Button::new("theme-toggle")
                                .ghost()
                                .small()
                                .icon(if is_dark { IconName::Sun } else { IconName::Moon })
                                .accessibility_label("Toggle dark mode")
                                .tooltip_with_action(
                                    if is_dark { "Light mode" } else { "Dark mode" },
                                    &ToggleTheme,
                                    Some(CONTEXT),
                                )
                                .on_click(cx.listener(|this, _, window, cx| this.toggle_theme(&ToggleTheme, window, cx))),
                        ),
                ),
        )
        .into_any_element()
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.current().route.section();
        let collapsed = self.sidebar_collapsed;
        Sidebar::new("app-sidebar")
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(collapsed)
            .w(px(224.))
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_8()
                                .flex_shrink_0()
                                .rounded(cx.theme().radius)
                                .bg(widgets::brand_navy())
                                .text_color(white())
                                .child("⚾"),
                        )
                        .when(!collapsed, |this| {
                            this.child(
                                v_flex()
                                    .min_w_0()
                                    .child(div().font_bold().text_sm().child("Baseball History"))
                                    .child(widgets::note("Lahman Baseball Database", cx)),
                            )
                        }),
                ),
            )
            .children(SIDEBAR.iter().map(|(group, sections)| {
                SidebarGroup::new(*group).child(SidebarMenu::new().children(sections.iter().map(|section| {
                    let section = *section;
                    SidebarMenuItem::new(section.label())
                        .icon(section.icon())
                        .active(section == active)
                        .on_click(self.nav.to(section.route()))
                })))
            }))
            .footer(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        Button::new("sidebar-api-docs")
                            .ghost()
                            .small()
                            .w_full()
                            .justify_start()
                            .icon(IconName::ExternalLink)
                            .when(!collapsed, |this| this.label("API documentation"))
                            .tooltip("Open the API documentation in your browser")
                            .on_click(|_, _, cx| cx.open_url("https://baseballhistoryproject.com/ApiDocs")),
                    )
                    .child(
                        Button::new("sidebar-github")
                            .ghost()
                            .small()
                            .w_full()
                            .justify_start()
                            .icon(IconName::Github)
                            .when(!collapsed, |this| this.label("Source on GitHub"))
                            .tooltip("Open the source code on GitHub")
                            .on_click(|_, _, cx| cx.open_url("https://github.com/cwoodruff/baseball-history")),
                    ),
            )
            .into_any_element()
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .h(px(28.))
            .px_3()
            .gap_3()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().status_bar)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Button::new("toggle-sidebar")
                            .ghost()
                            .xsmall()
                            .icon(IconName::PanelLeft)
                            .accessibility_label("Toggle sidebar")
                            .tooltip_with_action("Toggle sidebar", &ToggleSidebar, Some(CONTEXT))
                            .on_click(cx.listener(|this, _, window, cx| this.toggle_sidebar(&ToggleSidebar, window, cx))),
                    )
                    .child(format!("© {} Baseball History · Data from the", crate::catalog::LAST_SEASON + 1))
                    .child(
                        Button::new("lahman-link")
                            .link()
                            .xsmall()
                            .label("Lahman Baseball Database")
                            .on_click(|_, _, cx| cx.open_url("https://sabr.org/lahman-database/")),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("footer-about")
                            .ghost()
                            .xsmall()
                            .label("About")
                            .on_click(self.nav.to(Route::Reference(ReferencePage::About))),
                    )
                    .child(
                        Button::new("footer-licensing")
                            .link()
                            .xsmall()
                            .label("Licensing")
                            .on_click(|_, _, cx| cx.open_url("https://baseballhistoryproject.com/Licensing")),
                    )
                    .child(
                        Button::new("footer-privacy")
                            .link()
                            .xsmall()
                            .label("Privacy")
                            .on_click(|_, _, cx| cx.open_url("https://baseballhistoryproject.com/Privacy")),
                    ),
            )
        .into_any_element()
    }
}

fn group_label(label: &str, cx: &App) -> AnyElement {
    div()
        .px_3()
        .pt_2()
        .pb_1()
        .text_xs()
        .font_semibold()
        .text_color(cx.theme().muted_foreground)
        .child(label.to_string())
    .into_any_element()
}

fn search_row(
    id: SharedString,
    icon: IconName,
    title: String,
    subtitle: String,
    hof: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    div().px_1().child(
        Button::new(id)
            .ghost()
            .w_full()
            .h_auto()
            .on_click(on_click)
            .child(
                h_flex()
                    .w_full()
                    .py_1()
                    .gap_2()
                    .items_center()
                    .child(gpui_kit::component::Icon::new(icon).text_color(cx.theme().muted_foreground))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .items_start()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(div().text_sm().truncate().child(title))
                                    .when(hof, |this| this.child(widgets::hof_tag())),
                            )
                            .child(widgets::note(subtitle, cx)),
                    ),
            ),
    )
    .into_any_element()
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = self.current().page.view();
        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(Self::forward))
            .on_action(cx.listener(Self::reload))
            .on_action(cx.listener(Self::focus_search))
            .on_action(cx.listener(Self::toggle_theme))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::go_home))
            .on_action(cx.listener(Self::on_close_search))
            .on_mouse_up(MouseButton::Navigate(NavigationDirection::Back), cx.listener(|this, _, window, cx| this.back(&Back, window, cx)))
            .on_mouse_up(MouseButton::Navigate(NavigationDirection::Forward), cx.listener(|this, _, window, cx| this.forward(&Forward, window, cx)))
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_title_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(self.render_sidebar(cx))
                    .child(div().flex_1().min_w_0().h_full().child(page)),
            )
            .child(self.render_status_bar(cx))
        .into_any_element()
    }
}
