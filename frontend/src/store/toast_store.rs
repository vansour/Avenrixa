use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::cell::RefCell;
use std::rc::Rc;

/// Toast 消息类型
#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Info,
}

impl std::fmt::Display for ToastType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToastType::Success => write!(f, "success"),
            ToastType::Error => write!(f, "error"),
            ToastType::Info => write!(f, "info"),
        }
    }
}

/// Toast 消息
#[derive(Clone, Debug, PartialEq)]
pub struct ToastMessage {
    pub message: String,
    pub toast_type: ToastType,
    pub id: usize,
}

/// Toast Store - 提供访问 Toast Signal 的方法
#[derive(Clone)]
pub struct ToastStore {
    toasts: Rc<RefCell<Signal<Vec<ToastMessage>>>>,
    next_id: Rc<RefCell<usize>>,
}

impl ToastStore {
    pub fn new() -> Self {
        Self {
            toasts: Rc::new(RefCell::new(Signal::new(Vec::new()))),
            next_id: Rc::new(RefCell::new(0)),
        }
    }

    /// 获取 Toast Signal
    pub fn toasts(&self) -> Signal<Vec<ToastMessage>> {
        *self.toasts.borrow()
    }

    /// 显示成功消息
    pub fn show_success(&self, message: String) {
        self.add_toast(message, ToastType::Success);
    }

    /// 显示错误消息
    pub fn show_error(&self, message: String) {
        self.add_toast(message, ToastType::Error);
    }

    /// 显示信息消息
    pub fn show_info(&self, message: String) {
        self.add_toast(message, ToastType::Info);
    }

    /// 添加 Toast 到队列
    fn add_toast(&self, message: String, toast_type: ToastType) {
        let id = {
            let mut next_id = self.next_id.borrow_mut();
            let id = *next_id;
            *next_id += 1;
            id
        };

        {
            let mut toasts = self.toasts.borrow_mut();
            let mut write = toasts.write();
            write.push(ToastMessage { id, message, toast_type });
        }

        let id_for_removal = id;
        let toasts_rc = self.toasts.clone();
        spawn(async move {
            // Web 端没有 Tokio timer runtime，使用浏览器原生计时器避免 panic。
            TimeoutFuture::new(3_000).await;
            let mut toasts = toasts_rc.borrow_mut();
            let mut write = toasts.write();
            write.retain(|t| t.id != id_for_removal);
        });
    }
}

impl Default for ToastStore {
    fn default() -> Self {
        Self::new()
    }
}
