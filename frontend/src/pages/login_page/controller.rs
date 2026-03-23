use dioxus::prelude::*;

use crate::action_feedback::{set_action_error, spawn_tracked_action};
use crate::app_context::{use_auth_service, use_toast_store};
use crate::services::AuthService;
use crate::store::ToastStore;
use crate::types::api::{LoginRequest, RegisterRequest};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LoginMode {
    Login,
    Register,
    RequestReset,
    ConfirmReset,
    ConfirmEmailVerification,
}

#[cfg(target_arch = "wasm32")]
fn read_query_param(name: &str) -> Option<String> {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    let search = search.trim_start_matches('?');
    if search.is_empty() {
        return None;
    }

    for pair in search.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key == name {
            return urlencoding::decode(value)
                .ok()
                .map(|value| value.into_owned());
        }
    }

    None
}

#[cfg(not(target_arch = "wasm32"))]
fn read_query_param(_name: &str) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn clear_auth_query_from_location() {
    use wasm_bindgen::JsValue;

    if let Some(window) = web_sys::window()
        && let Ok(history) = window.history()
    {
        let pathname = window.location().pathname().ok().unwrap_or_default();
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&pathname));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_auth_query_from_location() {}

fn initial_mode() -> LoginMode {
    match read_query_param("mode").as_deref() {
        Some("verify-email") if read_query_param("token").is_some() => {
            LoginMode::ConfirmEmailVerification
        }
        _ if read_query_param("token").is_some() => LoginMode::ConfirmReset,
        _ => LoginMode::Login,
    }
}

#[derive(Clone)]
pub(super) struct LoginPageController {
    pub login_email: Signal<String>,
    pub register_email: Signal<String>,
    pub password: Signal<String>,
    pub register_confirm_password: Signal<String>,
    pub reset_email: Signal<String>,
    pub reset_token: Signal<String>,
    pub verification_token: Signal<String>,
    pub new_password: Signal<String>,
    pub confirm_password: Signal<String>,
    is_loading: Signal<bool>,
    error_message: Signal<String>,
    mode: Signal<LoginMode>,
    auth_service: AuthService,
    toast_store: ToastStore,
    mail_enabled: bool,
}

pub(super) fn use_login_page_controller(mail_enabled: bool) -> LoginPageController {
    let auth_service = use_auth_service();
    let toast_store = use_toast_store();

    let initial_token = read_query_param("token");
    let mode = use_signal(initial_mode);
    let login_email = use_signal(String::new);
    let register_email = use_signal(String::new);
    let password = use_signal(String::new);
    let register_confirm_password = use_signal(String::new);
    let reset_email = use_signal(String::new);
    let reset_token = use_signal(|| initial_token.clone().unwrap_or_default());
    let verification_token = use_signal(|| initial_token.clone().unwrap_or_default());
    let new_password = use_signal(String::new);
    let confirm_password = use_signal(String::new);
    let is_loading = use_signal(|| false);
    let error_message = use_signal(String::new);

    let controller = LoginPageController {
        login_email,
        register_email,
        password,
        register_confirm_password,
        reset_email,
        reset_token,
        verification_token,
        new_password,
        confirm_password,
        is_loading,
        error_message,
        mode,
        auth_service,
        toast_store,
        mail_enabled,
    };

    use_effect({
        let controller = controller.clone();
        move || {
            if controller.mail_enabled {
                return;
            }

            if matches!(
                controller.current_mode(),
                LoginMode::Register | LoginMode::RequestReset
            ) {
                controller.switch_to_login();
            }
        }
    });

    controller
}

impl LoginPageController {
    pub fn is_loading(&self) -> bool {
        (self.is_loading)()
    }

    pub fn error_message(&self) -> String {
        (self.error_message)()
    }

    pub fn current_mode(&self) -> LoginMode {
        (self.mode)()
    }

    pub fn show_login(&self) -> bool {
        self.current_mode() == LoginMode::Login
    }

    pub fn show_register(&self) -> bool {
        self.current_mode() == LoginMode::Register
    }

    pub fn show_request_reset(&self) -> bool {
        self.current_mode() == LoginMode::RequestReset
    }

    pub fn show_confirm_reset(&self) -> bool {
        self.current_mode() == LoginMode::ConfirmReset
    }

    pub fn show_confirm_email_verification(&self) -> bool {
        self.current_mode() == LoginMode::ConfirmEmailVerification
    }

    pub fn title(&self) -> &'static str {
        match self.current_mode() {
            LoginMode::RequestReset => "重置密码",
            LoginMode::ConfirmEmailVerification => "验证邮箱",
            LoginMode::Register => "创建账号",
            LoginMode::ConfirmReset => "设置新密码",
            LoginMode::Login => "登录控制台",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self.current_mode() {
            LoginMode::RequestReset => "输入邮箱，我们会向已配置的地址发送重置链接",
            LoginMode::ConfirmEmailVerification => "验证邮箱后即可使用新账号登录",
            LoginMode::Register => "注册后需要完成邮箱验证",
            LoginMode::ConfirmReset => "输入新密码以完成重置",
            LoginMode::Login if self.mail_enabled => "管理图片资产与访问权限",
            LoginMode::Login => "当前站点未启用邮件能力，仅支持已有账号直接登录",
        }
    }

    pub fn footer_tip(&self) -> &'static str {
        match self.current_mode() {
            LoginMode::Register => "注册后需要点击邮件中的验证链接激活账号。",
            LoginMode::ConfirmEmailVerification => "验证成功后即可返回登录。",
            _ if self.mail_enabled => "如果你还没有账号，可以先完成公开注册。",
            _ => "注册和密码找回入口会在邮件能力启用后开放。",
        }
    }

    pub fn switch_to_login(&self) {
        self.set_mode(LoginMode::Login);
    }

    pub fn switch_to_register(&self) {
        self.set_mode(LoginMode::Register);
    }

    pub fn switch_to_request_reset(&self) {
        self.set_mode(LoginMode::RequestReset);
    }

    pub fn submit_login(&self) {
        let email = (self.login_email)().trim().to_string();
        let password = (self.password)();

        if email.is_empty() || password.trim().is_empty() {
            set_action_error(self.error_message, &self.toast_store, "请输入邮箱和密码");
            return;
        }

        let auth_service = self.auth_service.clone();
        let toast_store = self.toast_store.clone();
        let mut login_email = self.login_email;
        let mut password_signal = self.password;
        let error_message = self.error_message;

        spawn_tracked_action(self.is_loading, self.error_message, async move {
            match auth_service.login(LoginRequest { email, password }).await {
                Ok(_) => {
                    login_email.set(String::new());
                    password_signal.set(String::new());
                    toast_store.show_success("登录成功".to_string());
                }
                Err(error) => {
                    set_action_error(error_message, &toast_store, format!("登录失败: {}", error));
                }
            }
        });
    }

    pub fn submit_register(&self) {
        let email = (self.register_email)().trim().to_string();
        let password = (self.password)();
        let confirm_password = (self.register_confirm_password)();

        if email.is_empty() || password.trim().is_empty() {
            set_action_error(self.error_message, &self.toast_store, "请填写邮箱和密码");
            return;
        }
        if password != confirm_password {
            set_action_error(
                self.error_message,
                &self.toast_store,
                "两次输入的密码不一致",
            );
            return;
        }

        let auth_service = self.auth_service.clone();
        let toast_store = self.toast_store.clone();
        let mut login_email = self.login_email;
        let mut register_email = self.register_email;
        let mut password_signal = self.password;
        let mut register_confirm_password = self.register_confirm_password;
        let mut mode = self.mode;
        let error_message = self.error_message;

        spawn_tracked_action(self.is_loading, self.error_message, async move {
            match auth_service
                .register(RegisterRequest { email, password })
                .await
            {
                Ok(_) => {
                    toast_store.show_success("注册成功，请查收邮箱完成验证".to_string());
                    login_email.set(String::new());
                    register_email.set(String::new());
                    password_signal.set(String::new());
                    register_confirm_password.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    set_action_error(error_message, &toast_store, format!("注册失败: {}", error));
                }
            }
        });
    }

    pub fn submit_request_reset(&self) {
        let email = (self.reset_email)().trim().to_string();
        if email.is_empty() {
            set_action_error(self.error_message, &self.toast_store, "请输入邮箱");
            return;
        }

        let auth_service = self.auth_service.clone();
        let toast_store = self.toast_store.clone();
        let mut reset_email = self.reset_email;
        let mut mode = self.mode;
        let error_message = self.error_message;

        spawn_tracked_action(self.is_loading, self.error_message, async move {
            match auth_service.request_password_reset(email).await {
                Ok(_) => {
                    toast_store.show_success("如果账号已配置找回邮箱，重置邮件已发送".to_string());
                    reset_email.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    set_action_error(
                        error_message,
                        &toast_store,
                        format!("发送重置邮件失败: {}", error),
                    );
                }
            }
        });
    }

    pub fn submit_confirm_reset(&self) {
        let token = (self.reset_token)().trim().to_string();
        let new_password = (self.new_password)();
        let confirm_password = (self.confirm_password)();

        if token.is_empty() {
            set_action_error(self.error_message, &self.toast_store, "重置令牌不能为空");
            return;
        }
        if new_password.trim().is_empty() {
            set_action_error(self.error_message, &self.toast_store, "请输入新密码");
            return;
        }
        if new_password != confirm_password {
            set_action_error(
                self.error_message,
                &self.toast_store,
                "两次输入的新密码不一致",
            );
            return;
        }

        let auth_service = self.auth_service.clone();
        let toast_store = self.toast_store.clone();
        let mut reset_token = self.reset_token;
        let mut new_password_signal = self.new_password;
        let mut confirm_password_signal = self.confirm_password;
        let mut mode = self.mode;
        let error_message = self.error_message;

        spawn_tracked_action(self.is_loading, self.error_message, async move {
            match auth_service
                .confirm_password_reset(token, new_password)
                .await
            {
                Ok(_) => {
                    toast_store.show_success("密码已重置，请使用新密码登录".to_string());
                    clear_auth_query_from_location();
                    reset_token.set(String::new());
                    new_password_signal.set(String::new());
                    confirm_password_signal.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    set_action_error(
                        error_message,
                        &toast_store,
                        format!("重置密码失败: {}", error),
                    );
                }
            }
        });
    }

    pub fn submit_confirm_email_verification(&self) {
        let token = (self.verification_token)().trim().to_string();
        if token.is_empty() {
            set_action_error(self.error_message, &self.toast_store, "验证令牌不能为空");
            return;
        }

        let auth_service = self.auth_service.clone();
        let toast_store = self.toast_store.clone();
        let mut verification_token = self.verification_token;
        let mut mode = self.mode;
        let error_message = self.error_message;

        spawn_tracked_action(self.is_loading, self.error_message, async move {
            match auth_service.confirm_email_verification(token).await {
                Ok(_) => {
                    toast_store.show_success("邮箱验证成功，请使用新账号登录".to_string());
                    clear_auth_query_from_location();
                    verification_token.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    set_action_error(
                        error_message,
                        &toast_store,
                        format!("邮箱验证失败: {}", error),
                    );
                }
            }
        });
    }

    fn set_mode(&self, next_mode: LoginMode) {
        let mut error_message = self.error_message;
        let mut mode = self.mode;
        error_message.set(String::new());
        mode.set(next_mode);
    }
}
