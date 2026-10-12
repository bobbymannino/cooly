use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString, assets::icon_assets};

// Icons used by Cooly's views that aren't in gpui-kit's default component bundle. Add an icon here when a view uses
// one that renders blank; embedding only these keeps the binary smaller than the full Lucide catalog (`AllAssets`).
icon_assets!(ExtraIcons, [LayoutGrid, LayoutList, LogOut]);

/// The app's asset source: gpui-kit's default component icons plus [`ExtraIcons`].
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // The default bundle errors on unknown paths, so it's asked last.
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = ExtraIcons.list(path)?;
        paths.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::assets::IconName;

    use super::*;

    #[test]
    fn loads_extra_and_default_icons() {
        for icon in [
            IconName::LayoutGrid,
            IconName::LayoutList,
            IconName::LogOut,
            IconName::RefreshCw,
            IconName::FolderOpen,
            IconName::Inbox,
        ] {
            assert!(Assets.load(&icon.path()).unwrap().is_some(), "{icon:?}");
        }
    }
}
