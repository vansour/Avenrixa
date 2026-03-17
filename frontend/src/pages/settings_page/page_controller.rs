use crate::app_context::{use_auth_store, use_settings_service, use_toast_store};
use crate::auth_session::{auth_session_expired_message, handle_auth_session_error};
use crate::services::SettingsService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::{AdminSettingsConfig, StorageBackendKind, TestS3StorageConfigRequest};
use crate::types::errors::AppError;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::S3ProviderPreset;
use super::state::SettingsFormState;
use super::summary::{is_current_s3_request_confirmed, requires_s3_test_confirmation};

const SETTINGS_LOAD_RETRY_DELAYS_MS: [u32; 3] = [0, 500, 1500];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum S3TestFeedbackTone {
    Neutral,
    Success,
    Error,
}

pub(crate) fn settings_auth_expired_message() -> String {
    auth_session_expired_message()
}

pub(crate) fn handle_settings_auth_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    err: &AppError,
) -> bool {
    handle_auth_session_error(auth_store, toast_store, err)
}

#[derive(Clone)]
pub(super) struct SettingsPageController {
    pub form: SettingsFormState,
    pub is_loading: Signal<bool>,
    pub is_saving: Signal<bool>,
    pub is_testing_s3: Signal<bool>,
    pub error_message: Signal<String>,
    pub loaded_config: Signal<Option<AdminSettingsConfig>>,
    pub last_tested_s3_request: Signal<Option<TestS3StorageConfigRequest>>,
    pub s3_test_feedback: Signal<String>,
    pub s3_test_feedback_tone: Signal<S3TestFeedbackTone>,
    reload_tick: Signal<u64>,
    last_loaded_tick: Signal<Option<u64>>,
    settings_service: SettingsService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    on_site_name_updated: EventHandler<String>,
    is_admin: bool,
}

pub(super) fn use_settings_page_controller(
    is_admin: bool,
    on_site_name_updated: EventHandler<String>,
) -> SettingsPageController {
    let settings_service = use_settings_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let is_loading = use_signal(|| true);
    let is_saving = use_signal(|| false);
    let is_testing_s3 = use_signal(|| false);
    let error_message = use_signal(String::new);
    let loaded_config = use_signal(|| None::<AdminSettingsConfig>);
    let last_tested_s3_request = use_signal(|| None::<TestS3StorageConfigRequest>);
    let s3_test_feedback = use_signal(String::new);
    let s3_test_feedback_tone = use_signal(|| S3TestFeedbackTone::Neutral);
    let reload_tick = use_signal(|| 0_u64);
    let last_loaded_tick = use_signal(|| None::<u64>);

    let site_name = use_signal(String::new);
    let storage_backend = use_signal(|| StorageBackendKind::Unknown);
    let local_storage_path = use_signal(String::new);
    let mail_enabled = use_signal(|| false);
    let mail_smtp_host = use_signal(String::new);
    let mail_smtp_port = use_signal(String::new);
    let mail_smtp_user = use_signal(String::new);
    let mail_smtp_password = use_signal(String::new);
    let mail_smtp_password_set = use_signal(|| false);
    let mail_from_email = use_signal(String::new);
    let mail_from_name = use_signal(String::new);
    let mail_link_base_url = use_signal(String::new);
    let s3_endpoint = use_signal(String::new);
    let s3_region = use_signal(String::new);
    let s3_bucket = use_signal(String::new);
    let s3_prefix = use_signal(String::new);
    let s3_access_key = use_signal(String::new);
    let s3_secret_key = use_signal(String::new);
    let s3_secret_key_set = use_signal(|| false);
    let s3_force_path_style = use_signal(|| true);
    let s3_provider_preset = use_signal(|| S3ProviderPreset::Other);
    let s3_provider_drafts = use_signal(std::collections::BTreeMap::new);

    let form = SettingsFormState {
        site_name,
        storage_backend,
        local_storage_path,
        mail_enabled,
        mail_smtp_host,
        mail_smtp_port,
        mail_smtp_user,
        mail_smtp_password,
        mail_smtp_password_set,
        mail_from_email,
        mail_from_name,
        mail_link_base_url,
        s3_endpoint,
        s3_region,
        s3_bucket,
        s3_prefix,
        s3_access_key,
        s3_secret_key,
        s3_secret_key_set,
        s3_force_path_style,
        s3_provider_preset,
        s3_provider_drafts,
    };

    let controller = SettingsPageController {
        form,
        is_loading,
        is_saving,
        is_testing_s3,
        error_message,
        loaded_config,
        last_tested_s3_request,
        s3_test_feedback,
        s3_test_feedback_tone,
        reload_tick,
        last_loaded_tick,
        settings_service,
        auth_store,
        toast_store,
        on_site_name_updated,
        is_admin,
    };

    use_effect({
        let controller = controller.clone();
        move || {
            let current_tick = (controller.reload_tick)();
            if (controller.last_loaded_tick)() == Some(current_tick) {
                return;
            }
            let mut last_loaded_tick = controller.last_loaded_tick;
            last_loaded_tick.set(Some(current_tick));
            let controller_for_reload = controller.clone();
            spawn(async move {
                controller_for_reload.reload_admin_settings().await;
            });
        }
    });

    controller
}

impl SettingsPageController {
    pub fn is_loading(&self) -> bool {
        (self.is_loading)()
    }

    pub fn is_saving(&self) -> bool {
        (self.is_saving)()
    }

    pub fn is_testing_s3(&self) -> bool {
        (self.is_testing_s3)()
    }

    pub fn error_message(&self) -> String {
        (self.error_message)()
    }

    pub fn loaded_config(&self) -> Option<AdminSettingsConfig> {
        (self.loaded_config)()
    }

    pub fn last_tested_s3_request(&self) -> Option<TestS3StorageConfigRequest> {
        (self.last_tested_s3_request)()
    }

    pub fn s3_test_feedback(&self) -> String {
        (self.s3_test_feedback)()
    }

    pub fn s3_test_feedback_tone(&self) -> S3TestFeedbackTone {
        (self.s3_test_feedback_tone)()
    }

    pub fn refresh(&self) {
        if self.is_loading() {
            return;
        }

        let mut reload_tick = self.reload_tick;
        reload_tick.set((self.reload_tick)().wrapping_add(1));
    }

    pub fn save(&self) {
        if self.is_saving() {
            return;
        }

        let requires_s3_test = self
            .loaded_config()
            .as_ref()
            .is_some_and(|config| requires_s3_test_confirmation(self.form, config));
        if requires_s3_test
            && !is_current_s3_request_confirmed(self.form, self.last_tested_s3_request())
        {
            let message = "请先完成 S3 连通性测试，再保存当前配置".to_string();
            let mut error_message = self.error_message;
            error_message.set(message.clone());
            self.toast_store.show_error(message);
            return;
        }

        if let Err(message) = self.form.validate() {
            let mut error_message = self.error_message;
            error_message.set(message.clone());
            self.toast_store.show_error(message);
            return;
        }

        let req = self.form.build_update_request(
            self.loaded_config()
                .as_ref()
                .map(|config| config.settings_version.clone()),
        );
        let controller = self.clone();
        spawn(async move {
            let mut is_saving = controller.is_saving;
            let mut error_message = controller.error_message;
            let mut loaded_config = controller.loaded_config;
            let mut last_tested_s3_request = controller.last_tested_s3_request;
            let mut s3_test_feedback = controller.s3_test_feedback;
            let mut s3_test_feedback_tone = controller.s3_test_feedback_tone;

            is_saving.set(true);
            error_message.set(String::new());

            match controller
                .settings_service
                .update_admin_settings_config(req)
                .await
            {
                Ok(config) => {
                    loaded_config.set(Some(config.clone()));
                    let mut form = controller.form;
                    form.apply_loaded_config(config.clone());
                    last_tested_s3_request.set(None);
                    s3_test_feedback.set(String::new());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);
                    controller
                        .on_site_name_updated
                        .call(config.site_name.clone());
                    controller
                        .toast_store
                        .show_success("设置已保存".to_string());
                }
                Err(err) => {
                    if handle_settings_auth_error(
                        &controller.auth_store,
                        &controller.toast_store,
                        &err,
                    ) {
                        error_message.set(settings_auth_expired_message());
                    } else {
                        let message = format!("保存设置失败: {}", err);
                        error_message.set(message.clone());
                        controller.toast_store.show_error(message);
                    }
                }
            }

            is_saving.set(false);
        });
    }

    pub fn test_s3(&self) {
        if self.is_testing_s3() || self.is_loading() || self.is_saving() {
            return;
        }

        if let Err(message) = self.form.validate_s3_for_test() {
            let mut error_message = self.error_message;
            let mut s3_test_feedback = self.s3_test_feedback;
            let mut s3_test_feedback_tone = self.s3_test_feedback_tone;

            error_message.set(message.clone());
            s3_test_feedback.set(message.clone());
            s3_test_feedback_tone.set(S3TestFeedbackTone::Error);
            self.toast_store.show_error(message);
            return;
        }

        let req = self.form.build_s3_test_request();
        let controller = self.clone();
        spawn(async move {
            let mut is_testing_s3 = controller.is_testing_s3;
            let mut error_message = controller.error_message;
            let mut last_tested_s3_request = controller.last_tested_s3_request;
            let mut s3_test_feedback = controller.s3_test_feedback;
            let mut s3_test_feedback_tone = controller.s3_test_feedback_tone;

            is_testing_s3.set(true);
            error_message.set(String::new());
            s3_test_feedback.set(String::new());
            s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);

            match controller
                .settings_service
                .test_s3_storage_config(req.clone())
                .await
            {
                Ok(response) => {
                    last_tested_s3_request.set(Some(req));
                    s3_test_feedback.set(response.message.clone());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Success);
                    controller.toast_store.show_success(response.message);
                }
                Err(err) => {
                    let message = format!("S3 测试失败: {}", err);
                    last_tested_s3_request.set(None);
                    s3_test_feedback.set(message.clone());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Error);
                    if handle_settings_auth_error(
                        &controller.auth_store,
                        &controller.toast_store,
                        &err,
                    ) {
                        error_message.set(settings_auth_expired_message());
                    } else {
                        error_message.set(message.clone());
                        controller.toast_store.show_error(message);
                    }
                }
            }

            is_testing_s3.set(false);
        });
    }

    async fn reload_admin_settings(&self) {
        if !self.is_admin {
            let mut is_loading = self.is_loading;
            let mut error_message = self.error_message;

            is_loading.set(false);
            error_message.set(String::new());
            return;
        }

        let mut is_loading = self.is_loading;
        let mut error_message = self.error_message;
        let mut loaded_config = self.loaded_config;
        let mut last_tested_s3_request = self.last_tested_s3_request;
        let mut s3_test_feedback = self.s3_test_feedback;
        let mut s3_test_feedback_tone = self.s3_test_feedback_tone;

        is_loading.set(true);
        error_message.set(String::new());
        let mut last_error = None;

        for delay_ms in SETTINGS_LOAD_RETRY_DELAYS_MS {
            if delay_ms > 0 {
                TimeoutFuture::new(delay_ms).await;
            }

            match self.settings_service.get_admin_settings_config().await {
                Ok(config) => {
                    loaded_config.set(Some(config.clone()));
                    let mut form = self.form;
                    form.apply_loaded_config(config);
                    last_tested_s3_request.set(None);
                    s3_test_feedback.set(String::new());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);
                    is_loading.set(false);
                    return;
                }
                Err(err) if err.should_redirect_login() => {
                    last_error = Some(err);
                    break;
                }
                Err(err) => last_error = Some(err),
            }
        }

        if let Some(err) = last_error {
            if handle_settings_auth_error(&self.auth_store, &self.toast_store, &err) {
                error_message.set(settings_auth_expired_message());
            } else {
                let message = format!("加载设置失败: {}", err);
                error_message.set(message.clone());
                self.toast_store.show_error(message);
            }
        }

        is_loading.set(false);
    }
}
