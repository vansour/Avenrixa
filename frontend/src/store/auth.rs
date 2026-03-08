use crate::types::api::UserResponse;
use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// 认证状态管理 Store
#[derive(Clone)]
pub struct AuthStore {
    user: Rc<RefCell<Signal<Option<UserResponse>>>>,
}

impl AuthStore {
    pub fn new() -> Self {
        Self {
            user: Rc::new(RefCell::new(Signal::new(None))),
        }
    }

    /// 获取当前用户
    pub fn user(&self) -> Option<UserResponse> {
        self.user.borrow().read().clone()
    }

    /// 获取当前用户引用
    pub fn user_as_ref(&self) -> Option<UserResponse> {
        self.user.borrow().read().as_ref().cloned()
    }

    /// 检查用户字段是否存在
    pub fn user_is_some(&self) -> bool {
        self.user.borrow().read().is_some()
    }

    /// 检查用户是否为空
    pub fn user_is_none(&self) -> bool {
        self.user.borrow().read().is_none()
    }

    /// 检查是否已认证
    pub fn is_authenticated(&self) -> bool {
        self.user.borrow().read().is_some()
    }

    /// 登录
    pub fn login(&self, user: UserResponse) {
        self.user.borrow_mut().set(Some(user));
    }

    /// 设置当前用户
    pub fn set_user(&self, user: UserResponse) {
        self.user.borrow_mut().set(Some(user));
    }

    /// 登出
    pub fn logout(&self) {
        self.user.borrow_mut().set(None);
    }
}

impl Default for AuthStore {
    fn default() -> Self {
        Self::new()
    }
}
