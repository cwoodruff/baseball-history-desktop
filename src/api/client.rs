//! Blocking HTTP client for the Baseball History Project API.
//!
//! Calls are synchronous by design: views run them on GPUI's background
//! executor (see [`crate::loader`]) so the UI thread never waits on the
//! network. Responses are cached (see [`super::cache`]).

use std::cell::Cell;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result, anyhow};
use serde::de::DeserializeOwned;

use super::cache::{self, Cache};
use super::models::*;

pub const DEFAULT_BASE_URL: &str = "https://baseballhistoryproject.com";

/// A cloneable handle to the shared HTTP agent.
#[derive(Clone)]
pub struct ApiClient {
    agent: ureq::Agent,
    base_url: Arc<str>,
    cache: Option<Arc<Cache>>,
}

/// Errors a view distinguishes from ordinary failures.
#[derive(Debug)]
pub enum ApiError {
    /// The record does not exist (HTTP 404).
    NotFound,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound => write!(f, "No record was found."),
        }
    }
}

impl std::error::Error for ApiError {}

thread_local! {
    /// When the oldest stale response served on this thread was fetched.
    static STALE: Cell<Option<SystemTime>> = const { Cell::new(None) };
}

/// Runs `f` and reports when the oldest stale cached response it was served
/// (because the service could not answer) was fetched, if any.
pub fn track_stale<R>(f: impl FnOnce() -> R) -> (R, Option<SystemTime>) {
    STALE.set(None);
    let result = f();
    (result, STALE.take())
}

fn note_stale(fetched: SystemTime) {
    let oldest = STALE.get().map_or(fetched, |seen| seen.min(fetched));
    STALE.set(Some(oldest));
}

/// Builds a query string, skipping parameters whose value is `None` or empty.
#[derive(Default)]
struct Query(Vec<(&'static str, String)>);

impl Query {
    fn new() -> Self {
        Self::default()
    }

    fn opt(mut self, key: &'static str, value: Option<impl ToString>) -> Self {
        if let Some(value) = value {
            let value = value.to_string();
            if !value.is_empty() {
                self.0.push((key, value));
            }
        }
        self
    }

    fn set(self, key: &'static str, value: impl ToString) -> Self {
        self.opt(key, Some(value))
    }

    fn encode(&self) -> String {
        if self.0.is_empty() {
            return String::new();
        }
        let pairs: Vec<String> = self
            .0
            .iter()
            .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
            .collect();
        format!("?{}", pairs.join("&"))
    }
}

fn seg(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(concat!(
                "baseball-history-desktop/",
                env!("CARGO_PKG_VERSION")
            ))
            .http_status_as_error(false)
            .build();
        let base_url: String = base_url.into();
        Self {
            agent: config.into(),
            base_url: base_url.trim_end_matches('/').into(),
            cache: None,
        }
    }

    pub fn with_cache(mut self, cache: Cache) -> Self {
        self.cache = Some(Arc::new(cache));
        self
    }

    /// Makes the next request for anything already cached ask the service.
    pub fn invalidate_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.invalidate();
        }
    }

    /// The process-wide client, honoring `BASEBALL_HISTORY_API_URL` when set.
    /// Responses are cached on disk unless `BASEBALL_HISTORY_NO_CACHE` is set.
    pub fn global() -> &'static ApiClient {
        static CLIENT: OnceLock<ApiClient> = OnceLock::new();
        CLIENT.get_or_init(|| {
            let base = std::env::var("BASEBALL_HISTORY_API_URL")
                .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
            let dir = std::env::var_os("BASEBALL_HISTORY_NO_CACHE")
                .is_none()
                .then(cache::default_dir)
                .flatten();
            ApiClient::new(base).with_cache(Cache::new(dir))
        })
    }

    /// Serves a fresh cached response, else asks the service, falling back to
    /// a stale cached response when the service cannot answer.
    fn get<T: DeserializeOwned>(&self, path: &str, query: Query) -> Result<T> {
        let url = format!("{}/api{}{}", self.base_url, path, query.encode());
        let parse = |body: &str| {
            serde_json::from_str::<T>(body)
                .with_context(|| format!("Unexpected response from {path}"))
        };
        let cached = self.cache.as_ref().and_then(|cache| cache.get(&url));
        if let Some(hit) = &cached
            && hit.fresh
            && let Ok(value) = parse(&hit.body)
        {
            return Ok(value);
        }
        match self.fetch(&url) {
            Ok(body) => {
                let value = parse(&body)?;
                if let Some(cache) = &self.cache {
                    cache.put(&url, body.into());
                }
                Ok(value)
            }
            Err(error) if is_not_found(&error) => Err(error),
            Err(error) => match cached.and_then(|hit| Some((parse(&hit.body).ok()?, hit))) {
                Some((value, hit)) => {
                    note_stale(hit.fetched);
                    Ok(value)
                }
                None => Err(error),
            },
        }
    }

    fn fetch(&self, url: &str) -> Result<String> {
        let mut response = self
            .agent
            .get(url)
            .header("Accept", "application/json")
            .call()
            .with_context(|| "Could not reach the Baseball History service.".to_string())?;
        let status = response.status().as_u16();
        if status == 404 {
            return Err(ApiError::NotFound.into());
        }
        if !(200..300).contains(&status) {
            return Err(anyhow!("The service responded with HTTP {status}."));
        }
        response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_string()
            .with_context(|| format!("Could not read the response from {url}"))
    }

    // MARK: Players

    pub fn players(&self, request: &PlayerQuery) -> Result<Paged<PlayerSummary>> {
        self.get("/players", request.query())
    }

    pub fn player(&self, id: &str) -> Result<PlayerDetail> {
        self.get(&format!("/players/{}", seg(id)), Query::new())
    }

    pub fn player_batting(&self, id: &str) -> Result<Vec<BattingSeason>> {
        self.get(&format!("/players/{}/batting", seg(id)), Query::new())
    }

    pub fn player_pitching(&self, id: &str) -> Result<Vec<PitchingSeason>> {
        self.get(&format!("/players/{}/pitching", seg(id)), Query::new())
    }

    pub fn player_fielding(&self, id: &str) -> Result<Vec<FieldingSeason>> {
        self.get(&format!("/players/{}/fielding", seg(id)), Query::new())
    }

    pub fn player_batting_advanced(&self, id: &str) -> Result<Vec<AdvancedBatting>> {
        self.get(
            &format!("/players/{}/batting/advanced", seg(id)),
            Query::new(),
        )
    }

    pub fn player_pitching_advanced(&self, id: &str) -> Result<Vec<AdvancedPitching>> {
        self.get(
            &format!("/players/{}/pitching/advanced", seg(id)),
            Query::new(),
        )
    }

    pub fn player_managing(&self, id: &str) -> Result<Vec<ManagerSeason>> {
        self.get(&format!("/players/{}/managing", seg(id)), Query::new())
    }

    pub fn player_all_star(&self, id: &str) -> Result<Vec<PlayerAllStar>> {
        self.get(&format!("/players/{}/all-star", seg(id)), Query::new())
    }

    pub fn player_awards(&self, id: &str) -> Result<Vec<PlayerAward>> {
        self.get(&format!("/players/{}/awards", seg(id)), Query::new())
    }

    pub fn player_postseason_batting(&self, id: &str) -> Result<Vec<PostseasonBatting>> {
        self.get(
            &format!("/players/{}/postseason/batting", seg(id)),
            Query::new(),
        )
    }

    pub fn player_postseason_pitching(&self, id: &str) -> Result<Vec<PostseasonPitching>> {
        self.get(
            &format!("/players/{}/postseason/pitching", seg(id)),
            Query::new(),
        )
    }

    // MARK: Teams

    pub fn franchises(
        &self,
        league: Option<&str>,
        active_only: bool,
    ) -> Result<Vec<FranchiseSummary>> {
        self.get(
            "/teams/franchises",
            Query::new()
                .opt("league", league)
                .opt("activeOnly", active_only.then_some(true)),
        )
    }

    pub fn franchise(&self, id: &str) -> Result<FranchiseDetail> {
        self.get(&format!("/teams/franchises/{}", seg(id)), Query::new())
    }

    pub fn team_season(&self, team_id: &str, lg_id: &str, year: i32) -> Result<TeamSeason> {
        self.get(
            &format!("/teams/seasons/{}/{}/{year}", seg(team_id), seg(lg_id)),
            Query::new(),
        )
    }

    // MARK: Leaders

    pub fn batting_leaders(&self, request: &LeaderRequest) -> Result<Paged<BattingLeader>> {
        self.get("/leaders/batting", request.query("minAb"))
    }

    pub fn pitching_leaders(&self, request: &LeaderRequest) -> Result<Paged<PitchingLeader>> {
        self.get("/leaders/pitching", request.query("minIp"))
    }

    // MARK: Hall of Fame

    pub fn hall_of_fame(
        &self,
        year: Option<i32>,
        category: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Paged<HallOfFamer>> {
        self.get(
            "/hall-of-fame",
            Query::new()
                .opt("year", year)
                .opt("category", category)
                .set("page", page)
                .set("pageSize", page_size),
        )
    }

    pub fn hall_of_fame_voting(&self, player_id: &str) -> Result<HallOfFameVoting> {
        self.get(
            &format!("/hall-of-fame/{}/voting", seg(player_id)),
            Query::new(),
        )
    }

    // MARK: Search

    pub fn search(&self, q: &str, limit: u32) -> Result<SearchResults> {
        self.get("/search", Query::new().set("q", q).set("limit", limit))
    }

    // MARK: Salaries

    pub fn player_salaries(&self, player_id: &str) -> Result<PlayerSalaryHistory> {
        self.get(
            &format!("/salaries/players/{}", seg(player_id)),
            Query::new(),
        )
    }

    pub fn team_payroll(&self, team_id: &str, year: i32) -> Result<TeamPayroll> {
        self.get(
            &format!("/salaries/teams/{}/{year}", seg(team_id)),
            Query::new(),
        )
    }

    pub fn salary_leaders(
        &self,
        year: Option<i32>,
        page: u32,
        page_size: u32,
    ) -> Result<Paged<SalaryEntry>> {
        self.get(
            "/salaries/leaders",
            Query::new()
                .opt("year", year)
                .set("page", page)
                .set("pageSize", page_size),
        )
    }

    // MARK: Parks

    pub fn parks(
        &self,
        state: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Paged<ParkSummary>> {
        self.get(
            "/parks",
            Query::new()
                .opt("state", state)
                .set("page", page)
                .set("pageSize", page_size),
        )
    }

    pub fn park(&self, key: &str) -> Result<ParkDetail> {
        self.get(&format!("/parks/{}", seg(key)), Query::new())
    }

    // MARK: Postseason

    pub fn postseason_series(
        &self,
        year: Option<i32>,
        round: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Paged<PostseasonSeries>> {
        self.get(
            "/postseason/series",
            Query::new()
                .opt("year", year)
                .opt("round", round)
                .set("page", page)
                .set("pageSize", page_size),
        )
    }

    pub fn postseason_year(&self, year: i32) -> Result<Vec<PostseasonSeries>> {
        self.get(&format!("/postseason/series/{year}"), Query::new())
    }

    // MARK: Awards

    pub fn award_winners(
        &self,
        award_id: Option<&str>,
        year: Option<i32>,
        lg_id: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Paged<AwardWinner>> {
        self.get(
            "/awards/winners",
            Query::new()
                .opt("awardId", award_id)
                .opt("year", year)
                .opt("lgId", lg_id)
                .set("page", page)
                .set("pageSize", page_size),
        )
    }

    pub fn award_voting(&self, award_id: &str, year: i32, lg_id: &str) -> Result<AwardVoting> {
        self.get(
            &format!("/awards/voting/{}/{year}/{}", seg(award_id), seg(lg_id)),
            Query::new(),
        )
    }

    // MARK: Managers

    /// Managers ranked by career wins, or by full name when `by_name`.
    pub fn managers(
        &self,
        q: Option<&str>,
        by_name: bool,
        page: u32,
        page_size: u32,
    ) -> Result<Paged<ManagerSummary>> {
        self.get(
            "/managers",
            Query::new()
                .opt("q", q)
                .set("sort", if by_name { "name" } else { "wins" })
                .set("page", page)
                .set("pageSize", page_size),
        )
    }

    pub fn manager(&self, id: &str) -> Result<ManagerDetail> {
        self.get(&format!("/managers/{}", seg(id)), Query::new())
    }

    // MARK: All-Star Games

    pub fn all_star_seasons(&self) -> Result<Vec<AllStarSeason>> {
        self.get("/all-star", Query::new())
    }

    pub fn all_star_year(&self, year: i32) -> Result<AllStarYear> {
        self.get(&format!("/all-star/{year}"), Query::new())
    }

    // MARK: Negro Leagues

    pub fn negro_leagues(&self) -> Result<Vec<NegroLeague>> {
        self.get("/negro-leagues", Query::new())
    }

    pub fn negro_league(&self, lg_id: &str) -> Result<NegroLeagueDetail> {
        self.get(&format!("/negro-leagues/{}", seg(lg_id)), Query::new())
    }

    pub fn negro_league_season(&self, lg_id: &str, year: i32) -> Result<NegroLeagueSeason> {
        self.get(
            &format!("/negro-leagues/{}/{year}", seg(lg_id)),
            Query::new(),
        )
    }

    // MARK: This Day in Baseball

    /// Birthdays, debuts and final games on `month`/`day` of any year.
    pub fn this_day(&self, month: u32, day: u32, limit: u32) -> Result<ThisDay> {
        self.get(
            "/this-day",
            Query::new()
                .set("date", format!("{month:02}-{day:02}"))
                .set("limit", limit),
        )
    }
}

/// Filters for the Players browser. A name query searches every letter.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerQuery {
    pub letter: char,
    pub q: Option<String>,
    /// Position code (`P`, `C`, `1B`, ..., `OF`, `DH`).
    pub pos: Option<String>,
    /// First year of a decade (1990 = the 1990s).
    pub era: Option<i32>,
    /// `name`, `hr`, `hits` or `games`.
    pub sort: String,
    pub page: u32,
    pub page_size: u32,
}

impl PlayerQuery {
    /// One page of players whose last name starts with `letter`, by name.
    pub fn letter(letter: char, page: u32, page_size: u32) -> Self {
        Self {
            letter,
            q: None,
            pos: None,
            era: None,
            sort: "name".into(),
            page,
            page_size,
        }
    }

    fn query(&self) -> Query {
        Query::new()
            .opt("letter", self.q.is_none().then_some(self.letter))
            .opt("q", self.q.as_deref())
            .opt("pos", self.pos.as_deref())
            .opt("era", self.era)
            .set("sort", &self.sort)
            .set("page", self.page)
            .set("pageSize", self.page_size)
    }
}

/// Filters shared by the batting and pitching leaderboards.
#[derive(Debug, Clone, PartialEq)]
pub struct LeaderRequest {
    pub stat: String,
    pub from_year: Option<i32>,
    pub to_year: Option<i32>,
    pub league: Option<String>,
    pub qualified: bool,
    /// Minimum at-bats (batting) or innings pitched (pitching).
    pub minimum: Option<i32>,
    pub single_season: bool,
    pub page: u32,
    pub page_size: u32,
}

impl LeaderRequest {
    fn query(&self, minimum_key: &'static str) -> Query {
        Query::new()
            .set("stat", &self.stat)
            .opt("fromYear", self.from_year)
            .opt("toYear", self.to_year)
            .opt("league", self.league.as_deref())
            .set("qualified", self.qualified)
            .opt(minimum_key, self.minimum)
            .set("singleSeason", self.single_season)
            .set("page", self.page)
            .set("pageSize", self.page_size)
    }
}

/// True when an error chain ends in a 404 from the service.
pub fn is_not_found(error: &anyhow::Error) -> bool {
    matches!(error.downcast_ref::<ApiError>(), Some(ApiError::NotFound))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_skips_empty_values_and_encodes() {
        let q = Query::new()
            .opt("letter", None::<char>)
            .opt("category", Some(""))
            .set("awardId", "Most Valuable Player")
            .set("page", 2);
        assert_eq!(q.encode(), "?awardId=Most%20Valuable%20Player&page=2");
    }

    #[test]
    fn leader_request_uses_endpoint_specific_minimum_key() {
        let request = LeaderRequest {
            stat: "ERA".into(),
            from_year: Some(1920),
            to_year: None,
            league: Some("AL".into()),
            qualified: true,
            minimum: Some(100),
            single_season: false,
            page: 1,
            page_size: 25,
        };
        let encoded = request.query("minIp").encode();
        assert!(encoded.contains("minIp=100"));
        assert!(encoded.contains("fromYear=1920"));
        assert!(!encoded.contains("toYear"));
    }

    #[test]
    fn player_query_drops_letter_while_searching() {
        let mut request = PlayerQuery {
            letter: 'R',
            q: None,
            pos: Some("OF".into()),
            era: Some(1920),
            sort: "hr".into(),
            page: 1,
            page_size: 48,
        };
        assert_eq!(
            request.query().encode(),
            "?letter=R&pos=OF&era=1920&sort=hr&page=1&pageSize=48"
        );
        request.q = Some("ruth".into());
        assert!(request.query().encode().starts_with("?q=ruth&pos=OF"));
    }

    #[test]
    fn models_deserialize_sampled_payloads() {
        let paged: Paged<PlayerSummary> = serde_json::from_str(
            r#"{"data":[{"playerId":"raabebr01","firstName":"Brian","lastName":"Raabe","debutYear":1995,"finalYear":1997,"isHallOfFamer":false,"totalGames":17,"totalHits":6,"totalHomeRuns":null,"lastTeamId":"COL","isPartialRecord":false}],"page":1,"pageSize":3,"totalCount":1329,"totalPages":443}"#,
        )
        .unwrap();
        assert_eq!(paged.data[0].total_home_runs, None);
        assert_eq!(paged.total_pages, 443);

        let voting: AwardVoting = serde_json::from_str(
            r#"{"awardId":"Cy Young Award","year":1990,"lgId":"AL","votes":[{"playerId":"welchbo01","playerName":"Bob Welch","pointsWon":107,"pointsMax":140,"votesFirst":"15","voteShare":76.4}]}"#,
        )
        .unwrap();
        assert_eq!(voting.votes[0].points_won, Some(107.0));
    }
}
