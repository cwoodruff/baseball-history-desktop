//! Static option lists for filters.
//!
//! The web app fills these dropdowns with `DISTINCT` queries against its own
//! database. The API has no endpoints for them, so they are fixed here from the
//! same Lahman data.

/// A filter option: the value sent to the API and the label shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub value: &'static str,
    pub label: &'static str,
}

const fn c(value: &'static str, label: &'static str) -> Choice {
    Choice { value, label }
}

/// Which way a leaderboard stat ranks its best values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatChoice {
    pub value: &'static str,
    pub label: &'static str,
    /// Rate stats default to the qualified minimum, like the web app.
    pub rate: bool,
}

const fn s(value: &'static str, label: &'static str, rate: bool) -> StatChoice {
    StatChoice { value, label, rate }
}

pub const BATTING_STATS: &[StatChoice] = &[
    s("hr", "Home Runs", false),
    s("h", "Hits", false),
    s("r", "Runs", false),
    s("rbi", "RBI", false),
    s("sb", "Stolen Bases", false),
    s("avg", "Batting Average", true),
    s("obp", "On-Base Percentage", true),
    s("slg", "Slugging Percentage", true),
    s("ops", "OPS", true),
    s("2b", "Doubles", false),
    s("3b", "Triples", false),
    s("tb", "Total Bases", false),
    s("bb", "Walks", false),
    s("g", "Games", false),
];

pub const PITCHING_STATS: &[StatChoice] = &[
    s("w", "Wins", false),
    s("era", "ERA", true),
    s("so", "Strikeouts", false),
    s("sv", "Saves", false),
    s("cg", "Complete Games", false),
    s("sho", "Shutouts", false),
    s("ip", "Innings Pitched", false),
    s("whip", "WHIP", true),
    s("k9", "K/9", true),
    s("wpct", "Win %", true),
    s("l", "Losses", false),
    s("g", "Games", false),
    s("gs", "Games Started", false),
];

/// Minimum thresholds. `None` means Qualified; `Some(0)` means no minimum.
pub const MIN_AB: &[(Option<i32>, &str)] = &[
    (None, "Qualified"),
    (Some(0), "No minimum"),
    (Some(100), "100 AB"),
    (Some(500), "500 AB"),
    (Some(1000), "1000 AB"),
    (Some(3000), "3000 AB"),
];

pub const MIN_IP: &[(Option<i32>, &str)] = &[
    (None, "Qualified"),
    (Some(0), "No minimum"),
    (Some(50), "50 IP"),
    (Some(100), "100 IP"),
    (Some(500), "500 IP"),
    (Some(1000), "1000 IP"),
];

pub const LEAGUES: &[Choice] = &[
    c("AL", "AL — American League"),
    c("NL", "NL — National League"),
    c("AA", "AA — American Association"),
    c("UA", "UA — Union Association"),
    c("PL", "PL — Players' League"),
    c("FL", "FL — Federal League"),
    c("NA", "NA — National Association"),
    c("NNL", "NNL — Negro National League (I)"),
    c("ECL", "ECL — Eastern Colored League"),
    c("ANL", "ANL — American Negro League"),
    c("EWL", "EWL — East-West League"),
    c("NSL", "NSL — Negro Southern League"),
    c("NN2", "NN2 — Negro National League (II)"),
    c("NAL", "NAL — Negro American League"),
];

pub const FIRST_SEASON: i32 = 1871;
pub const LAST_SEASON: i32 = 2025;
pub const FIRST_SALARY_SEASON: i32 = 1985;
pub const LAST_SALARY_SEASON: i32 = 2016;

/// Hall of Fame category cards (label, API value).
pub const HOF_CATEGORIES: &[Choice] = &[
    c("Player", "Players"),
    c("Manager", "Managers"),
    c("Pioneer/Executive", "Pioneers & Executives"),
    c("Umpire", "Umpires"),
];

/// Every category value the service stores, for the Category dropdown.
pub const HOF_CATEGORY_VALUES: &[&str] = &[
    "Executive",
    "Manager",
    "Pioneer",
    "Pioneer/Executive",
    "Player",
    "Umpire",
];

pub const POSTSEASON_ROUNDS: &[Choice] = &[
    c("WS", "World Series"),
    c("ALCS", "AL Championship Series"),
    c("NLCS", "NL Championship Series"),
    c("ALDS1", "AL Division Series 1"),
    c("ALDS2", "AL Division Series 2"),
    c("NLDS1", "NL Division Series 1"),
    c("NLDS2", "NL Division Series 2"),
    c("ALWC", "AL Wild Card"),
    c("ALWC1", "AL Wild Card 1"),
    c("ALWC2", "AL Wild Card 2"),
    c("NLWC", "NL Wild Card"),
    c("NLWC1", "NL Wild Card 1"),
    c("NLWC2", "NL Wild Card 2"),
];

/// A readable name for a postseason round code.
pub fn round_label(code: &str) -> String {
    if let Some(choice) = POSTSEASON_ROUNDS.iter().find(|r| r.value == code) {
        return choice.label.to_string();
    }
    match code {
        "AEDIV" | "AWDIV" => "AL Division Series".into(),
        "NEDIV" | "NWDIV" => "NL Division Series".into(),
        "CS" => "Championship Series".into(),
        "NWS" => "Negro World Series".into(),
        c if c.starts_with("ALWC") => "AL Wild Card".into(),
        c if c.starts_with("NLWC") => "NL Wild Card".into(),
        other => other.to_string(),
    }
}

/// Sort order within a postseason year: Wild Card, Division, LCS, World Series.
pub fn round_order(code: &str) -> u8 {
    if code.contains("WC") {
        0
    } else if code.contains("DS") || code.ends_with("DIV") {
        1
    } else if code.contains("CS") || code == "ALC" || code == "NLC" {
        2
    } else if code.contains("WS") {
        3
    } else {
        1
    }
}

/// Awards the service has ballot-by-ballot voting for.
pub const VOTED_AWARDS: &[&str] = &["Cy Young Award", "Most Valuable Player", "Rookie of the Year"];

pub const AWARDS: &[&str] = &[
    "ALCS MVP",
    "All-MLB Team - First Team",
    "All-MLB Team - Second Team",
    "All-Star Game MVP",
    "Babe Ruth Award",
    "Baseball Magazine All-Star",
    "Branch Rickey Award",
    "Comeback Player of the Year",
    "Cy Young Award",
    "Gold Glove",
    "Hank Aaron Award",
    "Hutch Award",
    "Lou Gehrig Memorial Award",
    "MLB Players Choice Comeback Player",
    "MLB Players Choice Man of the Year",
    "MLB Players Choice Outstanding Pitcher",
    "MLB Players Choice Outstanding Player",
    "MLB Players Choice Outstanding Rookie",
    "MLB Players Choice Player of the Year",
    "MLB Players Curt Flood Award",
    "Most Valuable Player",
    "NLCS MVP",
    "Outstanding DH Award",
    "Pitcher of the Month",
    "Pitching Triple Crown",
    "Platinum Glove",
    "Player of the Month",
    "Player of the Week",
    "Reliever of the Month",
    "Reliever of the Year",
    "Reliever of the Year Award",
    "Roberto Clemente Award",
    "Rookie of the Month",
    "Rookie of the Year",
    "Silver Slugger",
    "Triple Crown",
    "TSN All-Star",
    "TSN Comeback Player of the Year",
    "TSN Fireman of the Year",
    "TSN Guide MVP",
    "TSN Major League Player of the Year",
    "TSN Pitcher of the Year",
    "TSN Player of the Year",
    "TSN Reliever of the Year",
    "TSN Rookie of the Year",
    "TSN Rookie Pitcher of the Year",
    "TSN Rookie Player of the Year",
    "World Series MVP",
];

pub const AWARD_LEAGUES: &[Choice] = &[c("AL", "AL"), c("NL", "NL"), c("ML", "ML")];

/// Teams with salary data: (team ID, name, first year, last year).
pub const SALARY_TEAMS: &[(&str, &str, i32, i32)] = &[
    ("ANA", "Anaheim Angels", 1997, 2004),
    ("ARI", "Arizona Diamondbacks", 1998, 2016),
    ("ATL", "Atlanta Braves", 1985, 2016),
    ("BAL", "Baltimore Orioles", 1985, 2016),
    ("BOS", "Boston Red Sox", 1985, 2016),
    ("CAL", "California Angels", 1985, 1996),
    ("CHN", "Chicago Cubs", 1985, 2016),
    ("CHA", "Chicago White Sox", 1985, 2016),
    ("CIN", "Cincinnati Reds", 1985, 2016),
    ("CLE", "Cleveland Indians", 1985, 2016),
    ("COL", "Colorado Rockies", 1993, 2016),
    ("DET", "Detroit Tigers", 1985, 2016),
    ("FLO", "Florida Marlins", 1993, 2011),
    ("HOU", "Houston Astros", 1985, 2016),
    ("KCA", "Kansas City Royals", 1985, 2016),
    ("LAA", "Los Angeles Angels of Anaheim", 2005, 2016),
    ("LAN", "Los Angeles Dodgers", 1985, 2016),
    ("MIA", "Miami Marlins", 2012, 2016),
    ("MIL", "Milwaukee Brewers", 1998, 2016),
    ("ML4", "Milwaukee Brewers", 1985, 1997),
    ("MIN", "Minnesota Twins", 1985, 2016),
    ("MON", "Montreal Expos", 1985, 2004),
    ("NYN", "New York Mets", 1985, 2016),
    ("NYA", "New York Yankees", 1985, 2016),
    ("OAK", "Oakland Athletics", 1985, 2016),
    ("PHI", "Philadelphia Phillies", 1985, 2016),
    ("PIT", "Pittsburgh Pirates", 1985, 2016),
    ("SDN", "San Diego Padres", 1985, 2016),
    ("SFN", "San Francisco Giants", 1985, 2016),
    ("SEA", "Seattle Mariners", 1985, 2016),
    ("SLN", "St. Louis Cardinals", 1985, 2016),
    ("TBA", "Tampa Bay Rays", 1998, 2016),
    ("TEX", "Texas Rangers", 1985, 2016),
    ("TOR", "Toronto Blue Jays", 1985, 2016),
    ("WAS", "Washington Nationals", 2005, 2016),
];

pub const PARK_STATES: &[&str] = &[
    "AL", "AR", "AZ", "CA", "CDMX", "CO", "CT", "DC", "DE", "ENG", "FL", "GA", "HI", "IA", "IL",
    "IN", "KS", "KY", "LA", "MA", "MD", "MI", "MN", "MO", "NC", "NE", "New South Wales", "NJ",
    "Nuevo Leon", "NV", "NY", "OH", "OK", "ON", "PA", "QC", "RI", "Seoul", "TN", "Tokyo", "TX",
    "VA", "WA", "WI", "WV",
];

/// Decades offered by the era filter, newest first.
pub fn decades() -> Vec<i32> {
    let first = FIRST_SEASON / 10 * 10;
    let last = LAST_SEASON / 10 * 10;
    (0..=(last - first) / 10).map(|i| last - i * 10).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_sort_wild_card_first() {
        let mut rounds = vec!["WS", "ALCS", "ALDS1", "ALWC1"];
        rounds.sort_by_key(|r| round_order(r));
        assert_eq!(rounds, ["ALWC1", "ALDS1", "ALCS", "WS"]);
        assert_eq!(round_label("AEDIV"), "AL Division Series");
    }

    #[test]
    fn decades_cover_history() {
        let d = decades();
        assert_eq!(d.first(), Some(&2020));
        assert_eq!(d.last(), Some(&1870));
    }
}
