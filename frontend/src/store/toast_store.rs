use dioxus::prelude::*;
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

    /// 根据 ID 删除指定 Toast
    pub fn remove_toast(&self, id: usize) {
        let mut toasts = self.toasts.borrow_mut();
        let mut write = toasts.write();
        write.retain(|toast| toast.id != id);
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
            write.push(ToastMessage {
                id,
                message,
                toast_type,
            });
        }
    }
}

impl Default for ToastStore {
    fn default() -> Self {
        Self::new()
    }
}
