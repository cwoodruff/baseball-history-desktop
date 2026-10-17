# Baseball History Desktop: plan

A native, cross-platform (macOS, Windows, Linux) desktop client for the
[Baseball History Project](https://baseballhistoryproject.com). It is built in
Rust with [GPUI Kit](https://gpui-kit.com) (`gpui-kit` 0.7.3). It mirrors the
ASP.NET Core web app ([cwoodruff/baseball-history](https://github.com/cwoodruff/baseball-history)).
All of its data comes from the public REST API described by
`https://baseballhistoryproject.com/openapi/v1.json`.

## Scope rule: one-to-one, API-backed only

A web feature is ported when the API can supply its data, either directly or
by computing it exactly from API fields. Features the API cannot supply are
left out rather than faked.

| Web page / feature | Desktop | API source |
| --- | --- | --- |
| Home (hero counts, HR & Wins leaders, recent HOF, quick links) | ✅ | `/players`, `/teams/franchises`, `/hall-of-fame`, `/leaders/*` |
| Home "This Day in Baseball" | ❌ no endpoint | — |
| Global search (debounced, players + teams) and full Search page | ✅ | `/search` |
| Players A–Z browser, paged | ✅ | `/players?letter` |
| Player name/position/era/sort filters | ❌ API has no such params (name search goes through Search) | — |
| Player detail: info, teams, honors, career batting/pitching | ✅ | `/players/{id}`, `/awards` |
| Player tabs: Batting, Pitching (+ Advanced: K/9, BB/9, WHIP computed), Fielding (+ career by position, FPct computed), Postseason | ✅ | `/players/{id}/*` |
| Advanced Batting (PA, BABIP, OPS vs Lg), Managing tab, All-Star count | ❌ needs fields the API lacks | — |
| Teams & Franchises (league filter, active/historical, name + era filter) | ✅ (name/era filtered locally over the full list) | `/teams/franchises` |
| Franchise detail + season history | ✅ | `/teams/franchises/{id}` |
| Team season (badges, stat boxes, park, managers, batters, pitchers) | ✅ | `/teams/seasons/{t}/{lg}/{y}` |
| Ballparks (state filter, name/city filter) | ✅ (all 345 parks loaded, filtered locally) | `/parks` |
| Park detail (summary boxes, attendance chart, home teams, seasons) | ✅ | `/parks/{key}` |
| Batting / Pitching leaders (stat, years, league, min AB/IP, single season) | ✅ | `/leaders/batting`, `/leaders/pitching` |
| Hall of Fame (category cards, year/category filters) | ✅ | `/hall-of-fame` |
| Awards & Voting (players scope, voting race) | ✅ | `/awards/winners`, `/awards/voting/...` |
| Awards: managers scope | ❌ | — |
| Postseason results (year/round; rounds are the API's exact codes, e.g. ALDS1) | ✅ | `/postseason/series` |
| Salary explorer (year, team payroll banner) | ✅ | `/salaries/leaders`, `/salaries/teams/...` |
| HOF ballot history, player salary history | ❌ not shown by the web app (endpoints exist; the client wraps them) | `/hall-of-fame/{id}/voting`, `/salaries/players/{id}` |
| Compare two players | ✅ | `/search` + `/players/{id}` |
| About, Glossary, Surviving Records (static text) | ✅ | bundled Markdown |
| Managers, All-Star Games, Negro Leagues hub | ❌ no endpoints | — |
| API / MCP docs, GitHub, Lahman credit | opened in the browser | — |
| Dark mode toggle | ✅ GPUI Kit theme mode | — |

## UX: adapting web patterns to a desktop app

- **Shell:** a sidebar workspace. The left Sidebar holds the stable global
  navigation, grouped Browse / Stats / Tools / Reference. A toolbar sits above
  the content with Back/Forward, a breadcrumb title, global search, and the
  theme toggle. A status bar at the bottom carries the data credit.
- **Navigation:** history-based (⌘[ / ⌘], Alt+←/→), with ⌘F/Ctrl+F to focus
  search and ⌘R to reload the current view. Every in-app link is a Button or a
  table row, never link styling; external resources open in the browser.
- **Player modal → page:** the web's quick-look modal becomes direct
  navigation to the player page, so Back returns you to where you were. The
  content and the "View full page" outcome are the same.
- **Tables:** paged lists use the virtualized `DataTable`. Click selects a
  row; double-click or Enter opens it; right-click offers "Open". Short tables
  on detail pages use the stateless `Table`, with Buttons in the cells that
  navigate.
- **States:** each view has explicit Loading (spinner or skeleton), Empty
  (the web's messages), and Error (Alert with Retry) states. The UI thread
  never blocks; requests run on the background executor, and a superseded
  request is cancelled by dropping its task.
- **Formatting:** matches the web: `.342`, ERA `2.28`, `$33,000,000`,
  thousands separators, `—` for missing values. HOF and Partial Record badges
  are kept (the Partial Record badge has a tooltip and links to Surviving
  Records).
- **Theme:** the web's navy `#002D72` / red `#D50032` brand is used as the
  primary/accent over GPUI Kit's light and dark themes.

## Architecture

```
src/
  main.rs            bootstrap: assets, gpui_kit::init, theme, keybindings, window
  api/{mod,models,client}.rs   serde models + blocking ureq client (background thread)
  format.rs          web-matching number/date formatting (+ unit tests)
  catalog.rs         static option lists (stats, leagues, rounds, awards, teams, states)
  route.rs           Route enum, titles, sidebar section mapping
  loader.rs          Loadable<T> + spawn-on-background helper
  workspace.rs       shell: sidebar, toolbar, search popover, history, page host
  ui/                record_table (generic DataTable delegate), widgets (badges, stat boxes,
                     empty/error/loading, section cards, page header)
  pages/             home, search, players, player, teams, franchise, team_season,
                     parks, park, leaders (batting+pitching), hall_of_fame, awards,
                     postseason, salaries, compare, reference (About/Glossary/Surviving Records)
content/*.md         static reference text ported from the web app
```

State ownership: `Workspace` owns history and the current page entity. Each
page entity owns its filter `Entity<…State>` handles, its `Loadable` data and
its in-flight `Task`. Pages navigate through a cloneable `Navigator` handle,
a weak reference to the workspace that defers navigation to the next frame.

## Milestones

1. Crate scaffold, API client + models, formatting (unit-tested against sampled payloads).
2. Shell: sidebar, toolbar, history, search, theme, keybindings.
3. Shared UI: record table, badges, states.
4. Pages in order: Home → Players → Player → Teams → Franchise → Team Season →
   Leaders → Hall of Fame → Awards → Postseason → Salaries → Parks → Park →
   Search → Compare → Reference.
5. `cargo clippy`, `cargo test`, then a visual pass over every screen (light and dark) with the offscreen
   snapshot tool (`examples/snapshots.rs`) against the live API.
