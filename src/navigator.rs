use ruffle_frontend_utils::backends::navigator::NavigatorInterface;
use std::fs::File;
use std::path::Path;
use url::Url;

#[derive(Clone)]
pub struct AndroidNavigatorInterface;

// TODO: Prompt the user for these things!
impl NavigatorInterface for AndroidNavigatorInterface {
    /// Outbound navigation is intentionally disabled: this player is embedded,
    /// so a SWF must not be able to launch a browser intent. Re-enabling it
    /// should go through the `ask` prompt the TODO above calls for, not by
    /// restoring `webbrowser::open` unconditionally.
    fn navigate_to_website(&self, url: Url) {
        log::info!("Blocked navigation to {}", url);
    }

    async fn open_file(&self, path: &Path) -> std::io::Result<File> {
        File::open(path)
    }

    async fn confirm_socket(&self, _host: &str, _port: u16) -> bool {
        true
    }
}
