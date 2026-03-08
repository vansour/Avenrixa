use std::sync::Arc;
use parking_lot::RwLock;

/// Toast 消息类型
#[derive(Debug, Clone)]
pub enum ToastMsg {
    Success(String),
    Error(String),
    Info(String),
}

/// UI 状态管理 Store
#[derive(Clone)]
pub struct UIStore {
    sidebar_open: Arc<RwLock<bool>>,
    toast_message: Arc<RwLock<Option<ToastMsg>>>,
}

impl UIStore {
    pub fn new() -> Self {
        Self {
            sidebar_open: Arc::new(RwLock::new(false)),
            toast_message: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取侧边栏状态
    pub fn sidebar_open(&self) -> bool {
        *self.sidebar_open.read()
    }

    /// 切换侧边栏
    pub fn toggle_sidebar(&self) {
        let mut open = self.sidebar_open.write();
        *open = !*open;
    }

    /// 设置侧边栏状态
    pub fn set_sidebar_open(&self, open: bool) {
        *self.sidebar_open.write() = open;
    }

    /// 显示 Toast 消息
    pub fn show_toast(&self, msg: ToastMsg) {
        *self.toast_message.write() = Some(msg);
    }

    /// 获取 Toast 消息
    pub fn toast_message(&self) -> Option<ToastMsg> {
        self.toast_message.read().clone()
    }

    /// 清除 Toast 消息
    pub fn clear_toast(&self) {
        *self.toast_message.write() = None;
    }

    /// 检查 toast_message 是否为空
    pub fn toast_message_is_none(&self) -> bool {
        self.toast_message.read().is_none()
    }
}

impl Default for UIStore {
    fn default() -> Self {
        Self::new()
    }
}
