//! Blocking HTTP client for the Baseball History Project API.
//!
//! Calls are synchronous by design: views run them on GPUI's background
//! executor (see [`crate::loader`]) so the UI thread never waits on the
//! network.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use serde::de::DeserializeOwned;

use super::models::*;

pub const DEFAULT_BASE_URL: &str = "https://baseballhistoryproject.com";

/// A cloneable handle to the shared HTTP agent.
#[derive(Clone)]
pub struct ApiClient {
    agent: ureq::Agent,
    base_url: Arc<str>,
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
            .user_agent(concat!("baseball-history-desktop/", env!("CARGO_PKG_VERSION")))
            .http_status_as_error(false)
            .build();
        let base_url: String = base_url.into();
        Self {
            agent: config.into(),
            base_url: base_url.trim_end_matches('/').into(),
        }
    }

    /// The process-wide client, honoring `BASEBALL_HISTORY_API_URL` when set.
    pub fn global() -> &'static ApiClient {
        static CLIENT: OnceLock<ApiClient> = OnceLock::new();
        CLIENT.get_or_init(|| {
            let base = std::env::var("BASEBALL_HISTORY_API_URL")
                .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
            ApiClient::new(base)
        })
    }

    fn get<T: DeserializeOwned>(&self, path: &str, query: Query) -> Result<T> {
        let url = format!("{}/api{}{}", self.base_url, path, query.encode());
        let mut response = self
            .agent
            .get(&url)
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
            .read_json::<T>()
            .with_context(|| format!("Unexpected response from {path}"))
    }

    // MARK: Players

    pub fn players(&self, letter: Option<char>, page: u32, page_size: u32) -> Result<Paged<PlayerSummary>> {
        self.get(
            "/players",
            Query::new()
                .opt("letter", letter)
                .set("page", page)
                .set("pageSize", page_size),
        )
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

    pub fn player_awards(&self, id: &str) -> Result<Vec<PlayerAward>> {
        self.get(&format!("/players/{}/awards", seg(id)), Query::new())
    }

    pub fn player_postseason_batting(&self, id: &str) -> Result<Vec<PostseasonBatting>> {
        self.get(&format!("/players/{}/postseason/batting", seg(id)), Query::new())
    }

    pub fn player_postseason_pitching(&self, id: &str) -> Result<Vec<PostseasonPitching>> {
        self.get(&format!("/players/{}/postseason/pitching", seg(id)), Query::new())
    }

    // MARK: Teams

    pub fn franchises(&self, league: Option<&str>, active_only: bool) -> Result<Vec<FranchiseSummary>> {
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
        self.get(&format!("/hall-of-fame/{}/voting", seg(player_id)), Query::new())
    }

    // MARK: Search

    pub fn search(&self, q: &str, limit: u32) -> Result<SearchResults> {
        self.get("/search", Query::new().set("q", q).set("limit", limit))
    }

    // MARK: Salaries

    pub fn player_salaries(&self, player_id: &str) -> Result<PlayerSalaryHistory> {
        self.get(&format!("/salaries/players/{}", seg(player_id)), Query::new())
    }

    pub fn team_payroll(&self, team_id: &str, year: i32) -> Result<TeamPayroll> {
        self.get(&format!("/salaries/teams/{}/{year}", seg(team_id)), Query::new())
    }

    pub fn salary_leaders(&self, year: Option<i32>, page: u32, page_size: u32) -> Result<Paged<SalaryEntry>> {
        self.get(
            "/salaries/leaders",
            Query::new()
                .opt("year", year)
                .set("page", page)
                .set("pageSize", page_size),
        )
    }

    // MARK: Parks

    pub fn parks(&self, state: Option<&str>, page: u32, page_size: u32) -> Result<Paged<ParkSummary>> {
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
