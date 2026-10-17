//! Destinations in the app, mirroring the web app's routes.

use gpui_kit::assets::IconName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Home,
    Players { letter: char },
    Player { id: String },
    Teams,
    Franchise { id: String },
    TeamSeason { team_id: String, lg_id: String, year: i32 },
    Parks,
    Park { key: String },
    BattingLeaders,
    PitchingLeaders,
    HallOfFame { year: Option<i32>, category: Option<String> },
    Awards,
    Postseason { year: Option<i32> },
    Salaries,
    Search { query: String },
    Compare { first: Option<String>, second: Option<String> },
    Reference(ReferencePage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferencePage {
    About,
    Glossary,
    SurvivingRecords,
}

/// A top-level sidebar destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Home,
    Players,
    Teams,
    Parks,
    BattingLeaders,
    PitchingLeaders,
    HallOfFame,
    Awards,
    Postseason,
    Salaries,
    Compare,
    Search,
    About,
    Glossary,
    SurvivingRecords,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Home => "Home",
            Section::Players => "Players",
            Section::Teams => "Teams",
            Section::Parks => "Ballparks",
            Section::BattingLeaders => "Batting Leaders",
            Section::PitchingLeaders => "Pitching Leaders",
            Section::HallOfFame => "Hall of Fame",
            Section::Awards => "Awards & Voting",
            Section::Postseason => "Postseason",
            Section::Salaries => "Salaries",
            Section::Compare => "Compare Players",
            Section::Search => "Search",
            Section::About => "About",
            Section::Glossary => "Glossary",
            Section::SurvivingRecords => "Surviving Records",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Section::Home => IconName::House,
            Section::Players => IconName::Users,
            Section::Teams => IconName::Flag,
            Section::Parks => IconName::MapPin,
            Section::BattingLeaders => IconName::ChartColumn,
            Section::PitchingLeaders => IconName::ChartBar,
            Section::HallOfFame => IconName::Landmark,
            Section::Awards => IconName::Award,
            Section::Postseason => IconName::Trophy,
            Section::Salaries => IconName::DollarSign,
            Section::Compare => IconName::Columns2,
            Section::Search => IconName::Search,
            Section::About => IconName::Info,
            Section::Glossary => IconName::BookA,
            Section::SurvivingRecords => IconName::ScrollText,
        }
    }

    /// The route a sidebar click opens.
    pub fn route(self) -> Route {
        match self {
            Section::Home => Route::Home,
            Section::Players => Route::Players { letter: 'A' },
            Section::Teams => Route::Teams,
            Section::Parks => Route::Parks,
            Section::BattingLeaders => Route::BattingLeaders,
            Section::PitchingLeaders => Route::PitchingLeaders,
            Section::HallOfFame => Route::HallOfFame { year: None, category: None },
            Section::Awards => Route::Awards,
            Section::Postseason => Route::Postseason { year: None },
            Section::Salaries => Route::Salaries,
            Section::Compare => Route::Compare { first: None, second: None },
            Section::Search => Route::Search { query: String::new() },
            Section::About => Route::Reference(ReferencePage::About),
            Section::Glossary => Route::Reference(ReferencePage::Glossary),
            Section::SurvivingRecords => Route::Reference(ReferencePage::SurvivingRecords),
        }
    }
}

/// Sidebar groups in display order.
pub const SIDEBAR: &[(&str, &[Section])] = &[
    ("Browse", &[Section::Home, Section::Players, Section::Teams, Section::Parks]),
    (
        "Stats",
        &[
            Section::BattingLeaders,
            Section::PitchingLeaders,
            Section::HallOfFame,
            Section::Awards,
            Section::Postseason,
            Section::Salaries,
        ],
    ),
    ("Tools", &[Section::Search, Section::Compare]),
    ("Reference", &[Section::About, Section::Glossary, Section::SurvivingRecords]),
];

impl Route {
    /// The sidebar entry highlighted while this route is shown.
    pub fn section(&self) -> Section {
        match self {
            Route::Home => Section::Home,
            Route::Players { .. } | Route::Player { .. } => Section::Players,
            Route::Teams | Route::Franchise { .. } | Route::TeamSeason { .. } => Section::Teams,
            Route::Parks | Route::Park { .. } => Section::Parks,
            Route::BattingLeaders => Section::BattingLeaders,
            Route::PitchingLeaders => Section::PitchingLeaders,
            Route::HallOfFame { .. } => Section::HallOfFame,
            Route::Awards => Section::Awards,
            Route::Postseason { .. } => Section::Postseason,
            Route::Salaries => Section::Salaries,
            Route::Search { .. } => Section::Search,
            Route::Compare { .. } => Section::Compare,
            Route::Reference(ReferencePage::About) => Section::About,
            Route::Reference(ReferencePage::Glossary) => Section::Glossary,
            Route::Reference(ReferencePage::SurvivingRecords) => Section::SurvivingRecords,
        }
    }
}
