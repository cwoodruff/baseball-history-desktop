# Baseball History Desktop

A native desktop client for the [Baseball History Project](https://baseballhistoryproject.com), for macOS, Windows and Linux. It is built in Rust with [GPUI Kit](https://gpui-kit.com).

It mirrors the features of the [ASP.NET Core web app](https://github.com/cwoodruff/baseball-history). Its data comes from the project's public REST API ([OpenAPI document](https://baseballhistoryproject.com/openapi/v1.json)). See [PLAN.md](PLAN.md) for the feature-by-feature mapping, including which web features are left out because the API doesn't provide their data.

## Download

Get the latest version from the **[Releases page](https://github.com/cwoodruff/baseball-history-desktop/releases/latest)**:

| Platform | Download | Notes |
| --- | --- | --- |
| macOS 10.15+ (Apple Silicon and Intel) | `…-macos-universal.dmg` | Open the disk image and drag the app to Applications. |
| Windows 10+ (x64) | `…-windows-x86_64.msi` | Or the `.zip` with a standalone `.exe`. |
| Linux (x86_64) | `…-linux-x86_64.deb` | For Debian and Ubuntu (`sudo apt install ./baseball-history-*.deb`). Or the `.tar.gz` binary; it needs a Vulkan driver. |

The builds aren't code-signed. On macOS, right-click the app and choose **Open** the first time; on Windows, choose **More info → Run anyway** if SmartScreen warns. Each release lists the files' checksums in `SHA256SUMS.txt`.

## Features

- **Home:** dataset counts, career home-run and wins leaders, and recent Hall of Fame inductees.
- **Players:** an A–Z browser, plus a player page with bio, teams, honors, career totals, and Batting, Pitching (with K/9, BB/9, WHIP), Fielding and Postseason tabs.
- **Teams:** franchises filtered by league, name and era, each franchise's season history, and team-season pages with roster leaders.
- **Ballparks:** parks filtered by state, name and city, with an attendance chart and tenant history.
- **Batting and Pitching Leaders:** filter by stat, years, league, minimum AB/IP and single season; click a column header to rank by it.
- **Hall of Fame, Awards & Voting** (including voting races), **Postseason** results and the **Salary Explorer** (including team payrolls).
- **Global search** (debounced, live dropdown), a full **Search** page and **Compare Players**.
- **About, Glossary** and **Surviving Records** reference pages.
- Light and dark themes, back/forward history, and keyboard shortcuts.

## Build and run

You need a recent stable Rust toolchain (edition 2024). On Linux you also need the usual GPUI system libraries: X11/Wayland, xkbcommon, fontconfig and Vulkan.

```sh
cargo run --release
```

To build a macOS app bundle (`Baseball History.app`, plus a `.dmg`) with the bundle identifier `com.baseballhistoryproject.desktop`:

```sh
cargo install cargo-bundle
cargo bundle --release   # output: target/release/bundle/osx/ and target/release/bundle/dmg/
```

### Releases

Releases are built by `.github/workflows/release.yml`. To cut one, bump `version` in `Cargo.toml`, merge it, then push a matching tag:

```sh
git tag v0.1.1 && git push origin v0.1.1
```

The workflow builds the app on each platform and publishes these to a GitHub release, with `SHA256SUMS.txt`:

| Platform | Files |
| --- | --- |
| macOS (Apple Silicon + Intel) | `.dmg`, zipped `.app` |
| Windows (x64) | `.msi` installer, zipped `.exe` |
| Linux (x86_64) | `.deb`, `.tar.gz` binary |

None of the builds are code-signed. On macOS, right-click the app and choose **Open** on first launch; on Windows, choose **More info → Run anyway** if SmartScreen warns. Running the workflow manually builds the same files as workflow artifacts without publishing a release.

To point the app at another deployment of the API, set `BASEBALL_HISTORY_API_URL` (default `https://baseballhistoryproject.com`).

### Keyboard shortcuts

`Ctrl` on Windows and Linux, `⌘` on macOS:

| Shortcut | Action |
| --- | --- |
| `⌘F` / `⌘K` | Focus search |
| `⌘[` / `⌘]`, `Alt+←` / `Alt+→` | Back / Forward |
| `⌘R` / `F5` | Reload the current view |
| `⌘B` | Toggle the sidebar |
| `⌘⇧L` | Toggle dark mode |
| `⌘⇧H` | Home |
| `Esc` | Close the search results |

## Development

```sh
cargo test     # formatting, API model and computation tests
cargo clippy --all-targets
# Render screens offscreen to PNG (macOS/Linux), against the live API:
cargo run --example snapshots -- /tmp/shots [dark] home player:ruthba01 season:NYA:AL:1927 type:yank
```

Layout: `src/api` holds the models and blocking client (requests run on GPUI's background executor). `src/workspace.rs` is the app shell. `src/pages` has one module per screen, and `src/ui` holds the shared table, picker and widgets.

Data: the [Lahman Baseball Database](https://sabr.org/lahman-database/), licensed CC BY-SA 3.0.
