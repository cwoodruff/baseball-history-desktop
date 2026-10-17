//! Response models for the Baseball History Project REST API.
//!
//! The OpenAPI document (`/openapi/v1.json`) lists every route and parameter
//! but publishes no response schemas, so these shapes mirror the JSON the
//! live service returns. Every field the service may omit or null is an
//! `Option` so a partial historical record still deserializes.

use serde::Deserialize;

/// A page of results from any list endpoint that takes `page` / `pageSize`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paged<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total_count: u32,
    pub total_pages: u32,
}

impl<T> Default for Paged<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            page: 1,
            page_size: 0,
            total_count: 0,
            total_pages: 0,
        }
    }
}

// MARK: Players

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSummary {
    pub player_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub debut_year: Option<i32>,
    pub final_year: Option<i32>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
    pub total_games: Option<i64>,
    pub total_hits: Option<i64>,
    pub total_home_runs: Option<i64>,
    pub last_team_id: Option<String>,
    #[serde(default)]
    pub is_partial_record: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetail {
    pub player_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub given_name: Option<String>,
    pub height: Option<String>,
    pub weight: Option<String>,
    pub bats: Option<String>,
    pub throws: Option<String>,
    pub birth_date: Option<String>,
    pub birth_city: Option<String>,
    pub birth_state: Option<String>,
    pub birth_country: Option<String>,
    pub death_date: Option<String>,
    pub debut_year: Option<i32>,
    pub final_year: Option<i32>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
    pub hof_induction_year: Option<i32>,
    pub career_batting: Option<CareerBatting>,
    pub career_pitching: Option<CareerPitching>,
    #[serde(default)]
    pub teams: Vec<PlayerTeam>,
    #[serde(default)]
    pub is_partial_record: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerBatting {
    pub games: Option<i64>,
    pub at_bats: Option<i64>,
    pub runs: Option<i64>,
    pub hits: Option<i64>,
    pub doubles: Option<i64>,
    pub triples: Option<i64>,
    pub home_runs: Option<i64>,
    pub rbi: Option<i64>,
    pub stolen_bases: Option<i64>,
    pub walks: Option<i64>,
    pub strikeouts: Option<i64>,
    pub batting_average: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerPitching {
    pub games: Option<i64>,
    pub games_started: Option<i64>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub saves: Option<i64>,
    pub complete_games: Option<i64>,
    pub shutouts: Option<i64>,
    pub innings_pitched: Option<f64>,
    pub hits: Option<i64>,
    pub earned_runs: Option<i64>,
    pub home_runs: Option<i64>,
    pub walks: Option<i64>,
    pub strikeouts: Option<i64>,
    pub era: Option<f64>,
    pub whip: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerTeam {
    pub team_id: String,
    pub team_name: Option<String>,
    pub first_year: Option<i32>,
    pub last_year: Option<i32>,
    pub seasons: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattingSeason {
    pub year: i32,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub lg_id: Option<String>,
    pub games: Option<i64>,
    pub at_bats: Option<i64>,
    pub runs: Option<i64>,
    pub hits: Option<i64>,
    pub doubles: Option<i64>,
    pub triples: Option<i64>,
    pub home_runs: Option<i64>,
    pub rbi: Option<i64>,
    pub stolen_bases: Option<i64>,
    pub walks: Option<i64>,
    pub strikeouts: Option<i64>,
    pub batting_average: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchingSeason {
    pub year: i32,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub lg_id: Option<String>,
    pub games: Option<i64>,
    pub games_started: Option<i64>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub saves: Option<i64>,
    pub innings_pitched: Option<f64>,
    pub hits: Option<i64>,
    pub earned_runs: Option<i64>,
    pub strikeouts: Option<i64>,
    pub walks: Option<i64>,
    pub era: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldingSeason {
    pub year: i32,
    pub team_id: Option<String>,
    pub lg_id: Option<String>,
    pub position: Option<String>,
    pub games: Option<i64>,
    pub put_outs: Option<i64>,
    pub assists: Option<i64>,
    pub errors: Option<i64>,
    pub double_plays: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerAward {
    pub year: i32,
    pub award_id: String,
    pub lg_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostseasonBatting {
    pub year: i32,
    pub round: Option<String>,
    pub team_id: Option<String>,
    pub lg_id: Option<String>,
    pub games: Option<i64>,
    pub at_bats: Option<i64>,
    pub hits: Option<i64>,
    pub home_runs: Option<i64>,
    pub rbi: Option<i64>,
    pub batting_average: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostseasonPitching {
    pub year: i32,
    pub round: Option<String>,
    pub team_id: Option<String>,
    pub lg_id: Option<String>,
    pub games: Option<i64>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub saves: Option<i64>,
    pub innings_pitched: Option<f64>,
    pub strikeouts: Option<i64>,
    pub era: Option<f64>,
}

// MARK: Teams

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FranchiseSummary {
    pub franchise_id: String,
    pub franchise_name: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    pub first_year: Option<i32>,
    pub last_year: Option<i32>,
    pub total_seasons: Option<i64>,
    pub total_wins: Option<i64>,
    pub total_losses: Option<i64>,
    pub win_pct: Option<f64>,
    pub world_series_wins: Option<i64>,
    pub pennant_wins: Option<i64>,
    pub current_team_id: Option<String>,
    pub current_league: Option<String>,
    pub current_division: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FranchiseDetail {
    pub franchise_id: String,
    pub franchise_name: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    pub first_year: Option<i32>,
    pub last_year: Option<i32>,
    pub total_seasons: Option<i64>,
    pub total_wins: Option<i64>,
    pub total_losses: Option<i64>,
    pub win_pct: Option<f64>,
    pub world_series_wins: Option<i64>,
    pub pennant_wins: Option<i64>,
    #[serde(default)]
    pub seasons: Vec<FranchiseSeason>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FranchiseSeason {
    pub year: i32,
    pub team_id: String,
    pub team_name: Option<String>,
    pub lg_id: Option<String>,
    pub div_id: Option<String>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub win_pct: Option<f64>,
    pub rank: Option<i64>,
    #[serde(default)]
    pub won_division: bool,
    #[serde(default)]
    pub won_pennant: bool,
    #[serde(default)]
    pub won_world_series: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSeason {
    pub team_id: String,
    pub team_name: Option<String>,
    pub year: i32,
    pub lg_id: Option<String>,
    pub div_id: Option<String>,
    pub franchise_id: Option<String>,
    pub franchise_name: Option<String>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub win_pct: Option<f64>,
    pub rank: Option<i64>,
    #[serde(default)]
    pub won_division: bool,
    #[serde(default)]
    pub won_wild_card: bool,
    #[serde(default)]
    pub won_pennant: bool,
    #[serde(default)]
    pub won_world_series: bool,
    pub park_name: Option<String>,
    pub attendance: Option<i64>,
    pub batting: Option<TeamBatting>,
    pub pitching: Option<TeamPitching>,
    #[serde(default)]
    pub batters: Vec<TeamBatter>,
    #[serde(default)]
    pub pitchers: Vec<TeamPitcher>,
    #[serde(default)]
    pub managers: Vec<TeamManager>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamBatting {
    pub runs: Option<i64>,
    pub at_bats: Option<i64>,
    pub hits: Option<i64>,
    pub doubles: Option<i64>,
    pub triples: Option<i64>,
    pub home_runs: Option<i64>,
    pub walks: Option<i64>,
    pub strikeouts: Option<i64>,
    pub stolen_bases: Option<i64>,
    pub batting_average: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPitching {
    pub runs_allowed: Option<i64>,
    pub earned_runs: Option<i64>,
    pub era: Option<f64>,
    pub complete_games: Option<i64>,
    pub shutouts: Option<i64>,
    pub saves: Option<i64>,
    pub hits_allowed: Option<i64>,
    pub home_runs_allowed: Option<i64>,
    pub walks_allowed: Option<i64>,
    pub strikeouts_thrown: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamBatter {
    pub player_id: String,
    pub full_name: Option<String>,
    pub games: Option<i64>,
    pub at_bats: Option<i64>,
    pub hits: Option<i64>,
    pub home_runs: Option<i64>,
    pub rbi: Option<i64>,
    pub batting_average: Option<f64>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPitcher {
    pub player_id: String,
    pub full_name: Option<String>,
    pub games: Option<i64>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub saves: Option<i64>,
    pub strikeouts: Option<i64>,
    pub era: Option<f64>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamManager {
    pub player_id: String,
    pub full_name: Option<String>,
    pub games: Option<i64>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub order: Option<i64>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
}

// MARK: Leaders

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattingLeader {
    pub rank: i64,
    pub player_id: String,
    pub player_name: Option<String>,
    pub year: Option<i32>,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
    pub games: Option<i64>,
    pub at_bats: Option<i64>,
    pub runs: Option<i64>,
    pub hits: Option<i64>,
    pub doubles: Option<i64>,
    pub triples: Option<i64>,
    pub home_runs: Option<i64>,
    pub rbi: Option<i64>,
    pub stolen_bases: Option<i64>,
    pub walks: Option<i64>,
    pub batting_average: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchingLeader {
    pub rank: i64,
    pub player_id: String,
    pub player_name: Option<String>,
    pub year: Option<i32>,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
    pub games: Option<i64>,
    pub games_started: Option<i64>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub saves: Option<i64>,
    pub complete_games: Option<i64>,
    pub shutouts: Option<i64>,
    pub innings_pitched: Option<f64>,
    pub hits: Option<i64>,
    pub earned_runs: Option<i64>,
    pub home_runs: Option<i64>,
    pub walks: Option<i64>,
    pub strikeouts: Option<i64>,
    pub era: Option<f64>,
    pub whip: Option<f64>,
}

// MARK: Hall of Fame

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HallOfFamer {
    pub player_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub induction_year: Option<i32>,
    pub category: Option<String>,
    pub voted_by: Option<String>,
    pub vote_percentage: Option<f64>,
    pub debut_year: Option<i32>,
    pub final_year: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HallOfFameVoting {
    pub player_id: String,
    pub full_name: Option<String>,
    #[serde(default)]
    pub voting_history: Vec<HallOfFameBallot>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HallOfFameBallot {
    pub year: i32,
    pub voted_by: Option<String>,
    pub votes: Option<String>,
    pub ballots: Option<String>,
    pub vote_percentage: Option<f64>,
    #[serde(default)]
    pub inducted: bool,
}

// MARK: Search

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub players: Vec<SearchPlayer>,
    #[serde(default)]
    pub franchises: Vec<SearchFranchise>,
    #[serde(default)]
    pub total_player_count: u32,
    #[serde(default)]
    pub total_franchise_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPlayer {
    pub player_id: String,
    pub full_name: Option<String>,
    pub debut_year: Option<i32>,
    pub final_year: Option<i32>,
    #[serde(default)]
    pub is_hall_of_famer: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFranchise {
    pub franchise_id: String,
    pub franchise_name: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    pub first_year: Option<i32>,
    pub last_year: Option<i32>,
}

// MARK: Salaries

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSalaryHistory {
    pub player_id: String,
    pub full_name: Option<String>,
    #[serde(default)]
    pub seasons: Vec<SalarySeason>,
    pub career_total: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalarySeason {
    pub year: i32,
    pub team_id: Option<String>,
    pub lg_id: Option<String>,
    pub salary: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPayroll {
    pub year: i32,
    pub team_id: String,
    pub total_payroll: Option<i64>,
    pub player_count: Option<i64>,
    #[serde(default)]
    pub players: Vec<SalaryEntry>,
}

/// One player's salary for one season, used by the leaders and payroll views.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalaryEntry {
    pub year: i32,
    pub team_id: Option<String>,
    pub lg_id: Option<String>,
    pub player_id: String,
    pub player_name: Option<String>,
    pub salary: Option<i64>,
}

// MARK: Parks

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParkSummary {
    pub park_key: String,
    pub park_name: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParkDetail {
    pub park_key: String,
    pub park_name: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    #[serde(default)]
    pub seasons: Vec<ParkSeason>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParkSeason {
    pub year: i32,
    pub team_id: Option<String>,
    pub lg_id: Option<String>,
    pub games: Option<i64>,
    pub attendance: Option<i64>,
}

// MARK: Postseason

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostseasonSeries {
    pub year: i32,
    pub round: String,
    pub winner_team_id: Option<String>,
    pub winner_lg_id: Option<String>,
    pub loser_team_id: Option<String>,
    pub loser_lg_id: Option<String>,
    pub wins: Option<i64>,
    pub losses: Option<i64>,
    pub ties: Option<i64>,
}

// MARK: Awards

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwardWinner {
    pub player_id: String,
    pub player_name: Option<String>,
    pub award_id: String,
    pub year: i32,
    pub lg_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwardVoting {
    pub award_id: String,
    pub year: i32,
    pub lg_id: Option<String>,
    #[serde(default)]
    pub votes: Vec<AwardVote>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwardVote {
    pub player_id: String,
    pub player_name: Option<String>,
    pub points_won: Option<f64>,
    pub points_max: Option<f64>,
    pub votes_first: Option<String>,
    pub vote_share: Option<f64>,
}
