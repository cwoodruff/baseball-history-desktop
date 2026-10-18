//! One module per destination. Each page is an entity that owns its filters,
//! its loaded data and its in-flight requests, so returning to it through
//! history restores it exactly as it was left.

mod all_star;
mod awards;
mod compare;
mod franchise;
mod hall_of_fame;
mod home;
mod leaders;
pub(crate) mod manager;
mod managers;
mod negro_leagues;
mod park;
mod parks;
mod player;
mod players;
mod postseason;
mod reference;
mod salaries;
mod search;
mod team_season;
mod teams;

use gpui_kit::*;

use crate::nav::Navigator;
use crate::route::Route;

/// What the shell needs from every page.
pub trait Page: Render + Sized {
    /// The title shown in the title bar.
    fn title(&self, cx: &App) -> SharedString;
    /// Re-requests everything the page shows.
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>);
}

macro_rules! page_views {
    ($($variant:ident => $ty:ty),* $(,)?) => {
        /// A live page entity of any kind.
        pub enum PageView {
            $($variant(Entity<$ty>)),*
        }

        impl PageView {
            pub fn title(&self, cx: &App) -> SharedString {
                match self {
                    $(PageView::$variant(page) => page.read(cx).title(cx)),*
                }
            }

            pub fn reload(&self, window: &mut Window, cx: &mut App) {
                match self {
                    $(PageView::$variant(page) => page.update(cx, |page, cx| page.reload(window, cx))),*
                }
            }

            pub fn view(&self) -> AnyView {
                match self {
                    $(PageView::$variant(page) => page.clone().into()),*
                }
            }
        }
    };
}

page_views! {
    Home => home::HomePage,
    Players => players::PlayersPage,
    Player => player::PlayerPage,
    Teams => teams::TeamsPage,
    Franchise => franchise::FranchisePage,
    TeamSeason => team_season::TeamSeasonPage,
    Parks => parks::ParksPage,
    Park => park::ParkPage,
    Leaders => leaders::LeadersPage,
    HallOfFame => hall_of_fame::HallOfFamePage,
    Awards => awards::AwardsPage,
    Postseason => postseason::PostseasonPage,
    Salaries => salaries::SalariesPage,
    Search => search::SearchPage,
    Compare => compare::ComparePage,
    Managers => managers::ManagersPage,
    Manager => manager::ManagerPage,
    AllStar => all_star::AllStarPage,
    AllStarYear => all_star::AllStarYearPage,
    NegroLeagues => negro_leagues::NegroLeaguesPage,
    NegroLeague => negro_leagues::NegroLeaguePage,
    NegroLeagueSeason => negro_leagues::NegroLeagueSeasonPage,
    Reference => reference::ReferencePageView,
}

/// Creates the page for `route`.
pub fn build(route: &Route, nav: &Navigator, window: &mut Window, cx: &mut App) -> PageView {
    let nav = nav.clone();
    match route.clone() {
        Route::Home => PageView::Home(cx.new(|cx| home::HomePage::new(nav, window, cx))),
        Route::Players { letter } => {
            PageView::Players(cx.new(|cx| players::PlayersPage::new(letter, nav, window, cx)))
        }
        Route::Player { id } => {
            PageView::Player(cx.new(|cx| player::PlayerPage::new(id, nav, window, cx)))
        }
        Route::Teams => PageView::Teams(cx.new(|cx| teams::TeamsPage::new(nav, window, cx))),
        Route::Franchise { id } => {
            PageView::Franchise(cx.new(|cx| franchise::FranchisePage::new(id, nav, window, cx)))
        }
        Route::TeamSeason {
            team_id,
            lg_id,
            year,
        } => PageView::TeamSeason(
            cx.new(|cx| team_season::TeamSeasonPage::new(team_id, lg_id, year, nav, window, cx)),
        ),
        Route::Parks => PageView::Parks(cx.new(|cx| parks::ParksPage::new(nav, window, cx))),
        Route::Park { key } => {
            PageView::Park(cx.new(|cx| park::ParkPage::new(key, nav, window, cx)))
        }
        Route::BattingLeaders => PageView::Leaders(
            cx.new(|cx| leaders::LeadersPage::new(leaders::Kind::Batting, nav, window, cx)),
        ),
        Route::PitchingLeaders => PageView::Leaders(
            cx.new(|cx| leaders::LeadersPage::new(leaders::Kind::Pitching, nav, window, cx)),
        ),
        Route::HallOfFame { year, category } => PageView::HallOfFame(
            cx.new(|cx| hall_of_fame::HallOfFamePage::new(year, category, nav, window, cx)),
        ),
        Route::Awards => PageView::Awards(cx.new(|cx| awards::AwardsPage::new(nav, window, cx))),
        Route::Postseason { year } => PageView::Postseason(
            cx.new(|cx| postseason::PostseasonPage::new(year, nav, window, cx)),
        ),
        Route::Salaries => {
            PageView::Salaries(cx.new(|cx| salaries::SalariesPage::new(nav, window, cx)))
        }
        Route::Search { query } => {
            PageView::Search(cx.new(|cx| search::SearchPage::new(query, nav, window, cx)))
        }
        Route::Compare { first, second } => PageView::Compare(
            cx.new(|cx| compare::ComparePage::new(first, second, nav, window, cx)),
        ),
        Route::Managers => {
            PageView::Managers(cx.new(|cx| managers::ManagersPage::new(nav, window, cx)))
        }
        Route::Manager { id } => {
            PageView::Manager(cx.new(|cx| manager::ManagerPage::new(id, nav, window, cx)))
        }
        Route::AllStar => {
            PageView::AllStar(cx.new(|cx| all_star::AllStarPage::new(nav, window, cx)))
        }
        Route::AllStarYear { year } => PageView::AllStarYear(
            cx.new(|cx| all_star::AllStarYearPage::new(year, nav, window, cx)),
        ),
        Route::NegroLeagues => PageView::NegroLeagues(
            cx.new(|cx| negro_leagues::NegroLeaguesPage::new(nav, window, cx)),
        ),
        Route::NegroLeague { lg_id } => PageView::NegroLeague(
            cx.new(|cx| negro_leagues::NegroLeaguePage::new(lg_id, nav, window, cx)),
        ),
        Route::NegroLeagueSeason { lg_id, year } => PageView::NegroLeagueSeason(
            cx.new(|cx| negro_leagues::NegroLeagueSeasonPage::new(lg_id, year, nav, window, cx)),
        ),
        Route::Reference(page) => PageView::Reference(
            cx.new(|cx| reference::ReferencePageView::new(page, nav, window, cx)),
        ),
    }
}
