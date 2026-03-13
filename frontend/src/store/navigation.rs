use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashboardPage {
    Upload,
    History,
    Api,
    Settings,
}

impl DashboardPage {
    pub fn label(self) -> &'static str {
        match self {
            DashboardPage::Upload => "上传中心",
            DashboardPage::History => "历史图库",
            DashboardPage::Api => "API 接入",
            DashboardPage::Settings => "系统设置",
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
    Audit,
    Advanced,
}

#[derive(Clone)]
pub struct NavigationStore {
    current_page: Rc<RefCell<Signal<DashboardPage>>>,
    settings_anchor: Rc<RefCell<Signal<Option<SettingsAnchor>>>>,
}

impl NavigationStore {
    pub fn new() -> Self {
        Self {
            current_page: Rc::new(RefCell::new(Signal::new(DashboardPage::Upload))),
            settings_anchor: Rc::new(RefCell::new(Signal::new(None))),
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
    }

    pub fn open_settings(&self, anchor: SettingsAnchor) {
        self.current_page.borrow_mut().set(DashboardPage::Settings);
        self.settings_anchor.borrow_mut().set(Some(anchor));
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
