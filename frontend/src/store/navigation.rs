use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashboardPage {
    Upload,
    History,
    Api,
    Settings,
}

impl DashboardPage {
    pub fn slug(self) -> &'static str {
        match self {
            DashboardPage::Upload => "",
            DashboardPage::History => "history",
            DashboardPage::Api => "api",
            DashboardPage::Settings => "settings",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DashboardPage::Upload => "上传中心",
            DashboardPage::History => "历史图库",
            DashboardPage::Api => "API 接入",
            DashboardPage::Settings => "系统设置",
        }
    }

    #[cfg(any(target_arch = "wasm32", test))]
    fn from_slug(slug: &str) -> Option<Self> {
        match slug.trim().trim_matches('/') {
            "" | "upload" => Some(Self::Upload),
            "history" => Some(Self::History),
            "api" => Some(Self::Api),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAnchor {
    Account,
    General,
    Storage,
    Security,
    System,
    Maintenance,
    Users,
}

impl SettingsAnchor {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::General => "general",
            Self::Storage => "storage",
            Self::Security => "security",
            Self::System => "system",
            Self::Maintenance => "maintenance",
            Self::Users => "users",
        }
    }

    #[cfg(any(target_arch = "wasm32", test))]
    fn from_slug(slug: &str) -> Option<Self> {
        match slug.trim().to_ascii_lowercase().as_str() {
            "account" => Some(Self::Account),
            "general" => Some(Self::General),
            "storage" => Some(Self::Storage),
            "security" => Some(Self::Security),
            "system" => Some(Self::System),
            "maintenance" => Some(Self::Maintenance),
            "users" => Some(Self::Users),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NavigationSnapshot {
    base_path: String,
    page: DashboardPage,
    settings_anchor: Option<SettingsAnchor>,
}

#[derive(Clone)]
pub struct NavigationStore {
    current_page: Rc<RefCell<Signal<DashboardPage>>>,
    settings_anchor: Rc<RefCell<Signal<Option<SettingsAnchor>>>>,
    base_path: String,
    #[cfg(target_arch = "wasm32")]
    _browser_binding: Rc<BrowserNavigationBinding>,
}

impl NavigationStore {
    pub fn new() -> Self {
        let snapshot = current_navigation_snapshot();
        let current_page = Rc::new(RefCell::new(Signal::new(snapshot.page)));
        let settings_anchor = Rc::new(RefCell::new(Signal::new(snapshot.settings_anchor)));
        let base_path = snapshot.base_path;

        #[cfg(target_arch = "wasm32")]
        let browser_binding = bind_browser_navigation(
            current_page.clone(),
            settings_anchor.clone(),
            base_path.clone(),
        );

        Self {
            current_page,
            settings_anchor,
            base_path,
            #[cfg(target_arch = "wasm32")]
            _browser_binding: browser_binding,
        }
    }

    pub fn current_page(&self) -> DashboardPage {
        *self.current_page.borrow().read()
    }

    pub fn current_settings_anchor(&self) -> Option<SettingsAnchor> {
        *self.settings_anchor.borrow().read()
    }

    pub fn navigate(&self, page: DashboardPage) {
        self.current_page.borrow_mut().set(page);
        self.settings_anchor.borrow_mut().set(None);
        sync_browser_url(&self.base_path, page, None);
    }

    pub fn open_settings(&self, anchor: SettingsAnchor) {
        self.current_page.borrow_mut().set(DashboardPage::Settings);
        self.settings_anchor.borrow_mut().set(Some(anchor));
        sync_browser_url(&self.base_path, DashboardPage::Settings, Some(anchor));
    }

    pub fn reset(&self) {
        self.navigate(DashboardPage::Upload);
    }
}

impl Default for NavigationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
struct BrowserNavigationBinding {
    callback: Closure<dyn FnMut(web_sys::Event)>,
}

#[cfg(target_arch = "wasm32")]
impl Drop for BrowserNavigationBinding {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let _ = window.remove_event_listener_with_callback(
                "popstate",
                self.callback.as_ref().unchecked_ref(),
            );
        }
    }
}

fn current_navigation_snapshot() -> NavigationSnapshot {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            let pathname = location.pathname().unwrap_or_default();
            let search = location.search().unwrap_or_default();
            return parse_navigation_snapshot(&pathname, &search);
        }
    }

    NavigationSnapshot {
        base_path: String::new(),
        page: DashboardPage::Upload,
        settings_anchor: None,
    }
}

#[cfg(target_arch = "wasm32")]
fn bind_browser_navigation(
    current_page: Rc<RefCell<Signal<DashboardPage>>>,
    settings_anchor: Rc<RefCell<Signal<Option<SettingsAnchor>>>>,
    base_path: String,
) -> Rc<BrowserNavigationBinding> {
    let callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let snapshot = current_navigation_snapshot();
        if snapshot.base_path != base_path {
            return;
        }

        current_page.borrow_mut().set(snapshot.page);
        settings_anchor.borrow_mut().set(snapshot.settings_anchor);
    }) as Box<dyn FnMut(web_sys::Event)>);

    if let Some(window) = web_sys::window() {
        let _ =
            window.add_event_listener_with_callback("popstate", callback.as_ref().unchecked_ref());
    }

    Rc::new(BrowserNavigationBinding { callback })
}

#[cfg(any(target_arch = "wasm32", test))]
fn parse_navigation_snapshot(pathname: &str, search: &str) -> NavigationSnapshot {
    let trimmed_path = pathname.trim().trim_end_matches('/');
    let mut segments = trimmed_path
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();

    let page = segments
        .last()
        .and_then(|segment| DashboardPage::from_slug(segment))
        .unwrap_or(DashboardPage::Upload);
    if segments
        .last()
        .is_some_and(|segment| DashboardPage::from_slug(segment).is_some())
    {
        let _ = segments.pop();
    }

    let base_path = normalize_base_path(if segments.is_empty() {
        String::new()
    } else {
        format!("/{}", segments.join("/"))
    });

    let settings_anchor = if page == DashboardPage::Settings {
        parse_settings_anchor(search)
    } else {
        None
    };

    NavigationSnapshot {
        base_path,
        page,
        settings_anchor,
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn parse_settings_anchor(search: &str) -> Option<SettingsAnchor> {
    parse_search_param(search, "section").and_then(|value| SettingsAnchor::from_slug(&value))
}

#[cfg(any(target_arch = "wasm32", test))]
fn parse_search_param(search: &str, key: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter(|pair| !pair.trim().is_empty())
        .find_map(|pair| {
            let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
            let decoded_key = urlencoding::decode(raw_key).ok()?;
            if decoded_key != key {
                return None;
            }

            let decoded_value = urlencoding::decode(raw_value).ok()?;
            Some(decoded_value.into_owned())
        })
}

#[cfg(any(target_arch = "wasm32", test))]
fn normalize_base_path(base_path: String) -> String {
    let trimmed = base_path.trim().trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        String::new()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed)
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn build_navigation_url(
    base_path: &str,
    page: DashboardPage,
    settings_anchor: Option<SettingsAnchor>,
) -> String {
    let base_path = normalize_base_path(base_path.to_string());
    let path = match page {
        DashboardPage::Upload => {
            if base_path.is_empty() {
                "/".to_string()
            } else {
                base_path
            }
        }
        _ => format!("{}/{}", base_path, page.slug()),
    };

    if page == DashboardPage::Settings
        && let Some(anchor) = settings_anchor
    {
        return format!("{path}?section={}", anchor.slug());
    }

    path
}

fn sync_browser_url(base_path: &str, page: DashboardPage, settings_anchor: Option<SettingsAnchor>) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (base_path, page, settings_anchor);

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window()
            && let Ok(history) = window.history()
        {
            let url = build_navigation_url(base_path, page, settings_anchor);
            let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_navigation_snapshot_supports_settings_deep_link() {
        let snapshot = parse_navigation_snapshot("/settings", "?section=storage");

        assert_eq!(snapshot.base_path, "");
        assert_eq!(snapshot.page, DashboardPage::Settings);
        assert_eq!(snapshot.settings_anchor, Some(SettingsAnchor::Storage));
    }

    #[test]
    fn parse_navigation_snapshot_keeps_subpath_prefix() {
        let snapshot = parse_navigation_snapshot("/console/settings", "?section=users");

        assert_eq!(snapshot.base_path, "/console");
        assert_eq!(snapshot.page, DashboardPage::Settings);
        assert_eq!(snapshot.settings_anchor, Some(SettingsAnchor::Users));
    }

    #[test]
    fn build_navigation_url_formats_upload_and_settings_routes() {
        assert_eq!(
            build_navigation_url("/console", DashboardPage::Upload, None),
            "/console"
        );
        assert_eq!(
            build_navigation_url("/console", DashboardPage::History, None),
            "/console/history"
        );
        assert_eq!(
            build_navigation_url(
                "/console",
                DashboardPage::Settings,
                Some(SettingsAnchor::Storage),
            ),
            "/console/settings?section=storage"
        );
    }
}
