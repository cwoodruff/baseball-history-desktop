//! Asset source: GPUI Kit's default component icons plus the Lucide icons this
//! app uses that the default bundle leaves out.

use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

gpui_kit::assets::icon_assets!(
    ExtraIcons,
    [
        Award, BookA, ChartBar, ChartColumn, ChevronDown, ChevronLeft, ChevronRight, Columns2,
        DollarSign, ExternalLink, Flag, Github, House, Inbox, Info, Landmark, MapPin, Moon,
        PanelLeft, RefreshCw, ScrollText, Search, Sun, Trophy, User, Users, ArrowUpDown, X,
        Calendar, Medal, Star
    ]
);

pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match ExtraIcons.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut names = gpui_kit::assets::Assets.list(path)?;
        names.extend(ExtraIcons.list(path)?);
        names.sort();
        names.dedup();
        Ok(names)
    }
}
