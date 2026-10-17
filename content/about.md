## Overview

Baseball History is an interactive application for exploring over 150 years of Major League Baseball statistics, from 1871 to 2025. This desktop edition shows the same data as the [Baseball History Project](https://baseballhistoryproject.com) website, read live from its public REST API.

Key features include:

- **Player Browser:** browse players alphabetically, with career statistics.
- **Team Browser:** explore franchises and their season-by-season history.
- **Statistical Leaders:** batting and pitching leaderboards with advanced filters.
- **Hall of Fame:** inductees by year and category, with voting history.
- **Awards, Postseason and Salaries:** award winners and voting races, every postseason series, and salary data from 1985.
- **Global Search:** find players and teams across the entire history of the game.

## Data Source

The data is provided by the [Lahman Baseball Database](https://sabr.org/lahman-database/). Sean Lahman and his contributors have maintained this database of Major League Baseball statistics for many years. It is a standard resource for baseball researchers and enthusiasts.

**License information:** this project uses the Lahman Baseball Database, which is available under the [Creative Commons Attribution-ShareAlike 3.0 Unported License](https://creativecommons.org/licenses/by-sa/3.0/). Full source, license and attribution details are on the [Data Licensing & Attribution](https://baseballhistoryproject.com/Licensing) page.

## Data Scope

The database records raw, unadjusted totals. It has no park factors and no rate-adjusted metrics (no OPS+, ERA+ or similar). That means cross-era comparisons, such as a 1930 OPS against a 1968 OPS, are not adjusted for context: scoring environments, schedule lengths and rules all differed.

Negro Leagues records are partial by nature. Seasons ran roughly 60–80 league games and many box scores did not survive, so documented totals often understate those careers. Players for whom only a partial name survived carry a **Partial record** badge throughout the app. See *Surviving Records* in the sidebar for why these fragments survived the way they did.

## Source Code

This project is open source. The website is on GitHub at [github.com/cwoodruff/baseball-history](https://github.com/cwoodruff/baseball-history).

## Technology Stack

| Component | Technology |
| --- | --- |
| Desktop app | Rust, GPUI Kit (GPUI) |
| Data | Baseball History Project REST API |
| Web app & API | ASP.NET Core 10.0, Razor Pages, Entity Framework Core 10.0 |
| Database | PostgreSQL with the Lahman Baseball Database |
