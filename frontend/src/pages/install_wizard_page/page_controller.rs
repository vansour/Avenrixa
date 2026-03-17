use crate::app_context::{use_install_service, use_toast_store};
use crate::pages::settings_page::{
    SettingsFormState, default_mail_link_base_url, infer_s3_provider_preset,
};
use crate::services::InstallService;
use crate::store::ToastStore;
use crate::types::api::{
    AdminSettingsConfig, InstallBootstrapRequest, InstallBootstrapResponse, StorageBackendKind,
    StorageDirectoryEntry, TestS3StorageConfigRequest,
};
use base64::Engine;
use dioxus::html::FileData;
use dioxus::prelude::*;

use super::summary::{
    DEFAULT_INSTALL_STORAGE_BROWSER_PATH, InstallWizardStep, S3TestFeedbackTone,
    initial_local_storage_path, initial_mail_smtp_port, initial_site_name, initial_storage_backend,
    install_admin_submit_error, is_current_install_s3_request_confirmed,
};

#[derive(Clone)]
pub(super) struct InstallWizardController {
    pub form: SettingsFormState,
    pub admin_email: Signal<String>,
    pub admin_password: Signal<String>,
    pub confirm_password: Signal<String>,
    pub show_admin_password: Signal<bool>,
    pub show_confirm_password: Signal<bool>,
    pub selected_favicon: Signal<Option<FileData>>,
    pub current_step: Signal<InstallWizardStep>,
    pub is_installing: Signal<bool>,
    pub error_message: Signal<String>,
    pub success_message: Signal<String>,
    pub is_testing_s3: Signal<bool>,
    pub last_tested_s3_request: Signal<Option<TestS3StorageConfigRequest>>,
    pub s3_test_feedback: Signal<String>,
    pub s3_test_feedback_tone: Signal<S3TestFeedbackTone>,
    pub storage_browser_open: Signal<bool>,
    pub storage_browser_loading: Signal<bool>,
    pub storage_browser_error: Signal<String>,
    pub storage_browser_current_path: Signal<String>,
    pub storage_browser_parent_path: Signal<Option<String>>,
    pub storage_browser_directories: Signal<Vec<StorageDirectoryEntry>>,
    install_service: InstallService,
    toast_store: ToastStore,
    on_installed: EventHandler<InstallBootstrapResponse>,
}

pub(super) fn use_install_wizard_controller(
    initial_config: &AdminSettingsConfig,
    on_installed: EventHandler<InstallBootstrapResponse>,
) -> InstallWizardController {
    let install_service = use_install_service();
    let toast_store = use_toast_store();

    let site_name = use_signal({
        let initial = initial_site_name(initial_config);
        move || initial.clone()
    });
    let storage_backend = use_signal({
        let initial = initial_storage_backend(initial_config);
        move || initial
    });
    let local_storage_path = use_signal({
        let initial = initial_local_storage_path(initial_config);
        move || initial.clone()
    });
    let mail_enabled = use_signal({
        let initial = initial_config.mail_enabled;
        move || initial
    });
    let mail_smtp_host = use_signal({
        let initial = initial_config.mail_smtp_host.clone();
        move || initial.clone()
    });
    let mail_smtp_port = use_signal({
        let initial = initial_mail_smtp_port(initial_config);
        move || initial.clone()
    });
    let mail_smtp_user = use_signal({
        let initial = initial_config.mail_smtp_user.clone().unwrap_or_default();
        move || initial.clone()
    });
    let mail_smtp_password = use_signal(String::new);
    let mail_smtp_password_set = use_signal({
        let initial = initial_config.mail_smtp_password_set;
        move || initial
    });
    let mail_from_email = use_signal({
        let initial = initial_config.mail_from_email.clone();
        move || initial.clone()
    });
    let mail_from_name = use_signal({
        let initial = initial_config.mail_from_name.clone();
        move || initial.clone()
    });
    let mail_link_base_url = use_signal({
        let initial = default_mail_link_base_url(&initial_config.mail_link_base_url);
        move || initial.clone()
    });
    let s3_endpoint = use_signal({
        let initial = initial_config.s3_endpoint.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_region = use_signal({
        let initial = initial_config.s3_region.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_bucket = use_signal({
        let initial = initial_config.s3_bucket.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_prefix = use_signal({
        let initial = initial_config.s3_prefix.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_access_key = use_signal({
        let initial = initial_config.s3_access_key.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_secret_key = use_signal(String::new);
    let s3_secret_key_set = use_signal({
        let initial = initial_config.s3_secret_key_set;
        move || initial
    });
    let s3_force_path_style = use_signal({
        let initial = initial_config.s3_force_path_style;
        move || initial
    });
    let s3_provider_preset = use_signal({
        let initial = infer_s3_provider_preset(
            initial_config.s3_endpoint.as_deref(),
            initial_config.s3_region.as_deref(),
            initial_config.s3_force_path_style,
        );
        move || initial
    });
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

    let admin_email = use_signal(String::new);
    let admin_password = use_signal(String::new);
    let confirm_password = use_signal(String::new);
    let show_admin_password = use_signal(|| false);
    let show_confirm_password = use_signal(|| false);
    let selected_favicon = use_signal(|| None::<FileData>);
    let current_step = use_signal(|| InstallWizardStep::Admin);
    let is_installing = use_signal(|| false);
    let error_message = use_signal(String::new);
    let success_message = use_signal(String::new);
    let is_testing_s3 = use_signal(|| false);
    let last_tested_s3_request = use_signal(|| None::<TestS3StorageConfigRequest>);
    let s3_test_feedback = use_signal(String::new);
    let s3_test_feedback_tone = use_signal(|| S3TestFeedbackTone::Neutral);
    let storage_browser_open = use_signal(|| false);
    let storage_browser_loading = use_signal(|| false);
    let storage_browser_error = use_signal(String::new);
    let storage_browser_current_path =
        use_signal(|| DEFAULT_INSTALL_STORAGE_BROWSER_PATH.to_string());
    let storage_browser_parent_path = use_signal(|| None::<String>);
    let storage_browser_directories = use_signal(Vec::<StorageDirectoryEntry>::new);

    InstallWizardController {
        form,
        admin_email,
        admin_password,
        confirm_password,
        show_admin_password,
        show_confirm_password,
        selected_favicon,
        current_step,
        is_installing,
        error_message,
        success_message,
        is_testing_s3,
        last_tested_s3_request,
        s3_test_feedback,
        s3_test_feedback_tone,
        storage_browser_open,
        storage_browser_loading,
        storage_browser_error,
        storage_browser_current_path,
        storage_browser_parent_path,
        storage_browser_directories,
        install_service,
        toast_store,
        on_installed,
    }
}

impl InstallWizardController {
    pub fn is_installing(&self) -> bool {
        (self.is_installing)()
    }

    pub fn is_testing_s3(&self) -> bool {
        (self.is_testing_s3)()
    }

    pub fn pick_favicon(&self, event: Event<FormData>) {
        let mut selected_favicon = self.selected_favicon;
        let mut files = event.files().into_iter();
        match files.next() {
            Some(file) => selected_favicon.set(Some(file)),
            None => selected_favicon.set(None),
        }
    }

    pub fn set_storage_backend(&self, backend: StorageBackendKind) {
        let mut storage_backend = self.form.storage_backend;
        let mut last_tested_s3_request = self.last_tested_s3_request;
        let mut s3_test_feedback = self.s3_test_feedback;
        let mut s3_test_feedback_tone = self.s3_test_feedback_tone;
        let mut storage_browser_open = self.storage_browser_open;
        let mut storage_browser_error = self.storage_browser_error;

        storage_backend.set(backend);
        last_tested_s3_request.set(None);
        s3_test_feedback.set(String::new());
        s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);
        storage_browser_open.set(false);
        storage_browser_error.set(String::new());
    }

    pub fn install(&self) {
        if self.is_installing() || self.is_testing_s3() {
            return;
        }

        let email = (self.admin_email)().trim().to_string();
        let password = (self.admin_password)();
        let confirm = (self.confirm_password)();
        if let Some(message) =
            install_admin_submit_error(email.as_str(), password.as_str(), confirm.as_str())
        {
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
        if self.form.is_s3_backend()
            && !is_current_install_s3_request_confirmed(self.form, (self.last_tested_s3_request)())
        {
            let message = "请先完成 S3 连通性测试，再继续安装".to_string();
            let mut error_message = self.error_message;
            error_message.set(message.clone());
            self.toast_store.show_error(message);
            return;
        }

        let controller = self.clone();
        let req_config = self.form.build_update_request(None);
        let favicon_file = (self.selected_favicon)();
        spawn(async move {
            let mut is_installing = controller.is_installing;
            let mut error_message = controller.error_message;
            let mut success_message = controller.success_message;

            is_installing.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            let favicon_data_url = match favicon_file {
                Some(file) => match favicon_file_to_data_url(file).await {
                    Ok(data_url) => Some(data_url),
                    Err(message) => {
                        error_message.set(message.clone());
                        controller.toast_store.show_error(message);
                        is_installing.set(false);
                        return;
                    }
                },
                None => None,
            };

            let request = InstallBootstrapRequest {
                admin_email: email,
                admin_password: password,
                favicon_data_url,
                config: req_config,
            };

            match controller
                .install_service
                .bootstrap_installation(request)
                .await
            {
                Ok(response) => {
                    let message = "安装完成，已自动登录管理员账户".to_string();
                    success_message.set(message.clone());
                    controller.toast_store.show_success(message);
                    controller.on_installed.call(response);
                }
                Err(err) => {
                    let message = format!("安装失败: {}", err);
                    error_message.set(message.clone());
                    controller.toast_store.show_error(message);
                }
            }

            is_installing.set(false);
        });
    }

    pub fn test_s3(&self) {
        if self.is_installing() || self.is_testing_s3() {
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

        let controller = self.clone();
        let req = self.form.build_s3_test_request();
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
                .install_service
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
                    error_message.set(message.clone());
                    s3_test_feedback.set(message.clone());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Error);
                    controller.toast_store.show_error(message);
                }
            }

            is_testing_s3.set(false);
        });
    }

    pub fn open_storage_browser(&self) {
        let mut storage_browser_open = self.storage_browser_open;
        storage_browser_open.set(true);
        self.browse_storage_path((self.form.local_storage_path)());
    }

    pub fn close_storage_browser(&self) {
        let mut storage_browser_open = self.storage_browser_open;
        storage_browser_open.set(false);
    }

    pub fn browse_storage_parent(&self) {
        if let Some(parent_path) = (self.storage_browser_parent_path)() {
            self.browse_storage_path(parent_path);
        }
    }

    pub fn browse_storage_path(&self, requested_path: String) {
        let controller = self.clone();
        let requested_path = requested_path.trim().to_string();
        let requested_path = if requested_path.is_empty() {
            DEFAULT_INSTALL_STORAGE_BROWSER_PATH.to_string()
        } else {
            requested_path
        };

        spawn(async move {
            let mut storage_browser_loading = controller.storage_browser_loading;
            let mut storage_browser_error = controller.storage_browser_error;
            let mut storage_browser_current_path = controller.storage_browser_current_path;
            let mut storage_browser_parent_path = controller.storage_browser_parent_path;
            let mut storage_browser_directories = controller.storage_browser_directories;

            storage_browser_loading.set(true);
            storage_browser_error.set(String::new());

            let response = controller
                .install_service
                .browse_storage_directories(Some(requested_path.as_str()))
                .await;

            match response {
                Ok(response) => {
                    storage_browser_current_path.set(response.current_path);
                    storage_browser_parent_path.set(response.parent_path);
                    storage_browser_directories.set(response.directories);
                }
                Err(error) => {
                    storage_browser_error.set(format!("读取目录失败：{}", error));
                }
            }

            storage_browser_loading.set(false);
        });
    }

    pub fn select_current_storage_directory(&self) {
        let mut local_storage_path = self.form.local_storage_path;
        let mut storage_browser_open = self.storage_browser_open;
        local_storage_path.set((self.storage_browser_current_path)());
        storage_browser_open.set(false);
    }
}

async fn favicon_file_to_data_url(file: FileData) -> Result<String, String> {
    let mime = infer_favicon_mime(&file);
    let bytes = file
        .read_bytes()
        .await
        .map_err(|err| format!("读取网站图标失败: {}", err))?;
    if bytes.is_empty() {
        return Err("网站图标内容为空".to_string());
    }

    Ok(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn infer_favicon_mime(file: &FileData) -> &'static str {
    if let Some(content_type) = file.content_type() {
        match content_type.trim().to_ascii_lowercase().as_str() {
            "image/x-icon" | "image/vnd.microsoft.icon" | "image/ico" => {
                return "image/x-icon";
            }
            "image/png" => return "image/png",
            "image/svg+xml" => return "image/svg+xml",
            "image/webp" => return "image/webp",
            "image/jpeg" | "image/jpg" => return "image/jpeg",
            _ => {}
        }
    }

    let filename = file.name().to_ascii_lowercase();
    if filename.ends_with(".ico") {
        "image/x-icon"
    } else if filename.ends_with(".svg") {
        "image/svg+xml"
    } else if filename.ends_with(".webp") {
        "image/webp"
    } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    }
}
