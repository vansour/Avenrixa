use crate::components::ConfirmationTone;
use crate::types::api::{
    BackupDatabaseFamily, BackupObjectRollbackAnchor, BackupObjectRollbackStrategy,
    BackupRestorePrecheckResponse, BackupRestoreStorageSummary, BackupSemantics,
};

use super::models::ConfirmationPlan;

pub(crate) fn restore_confirmation_plan(
    precheck: &BackupRestorePrecheckResponse,
) -> ConfirmationPlan {
    let mut consequences = vec![
        format!(
            "目标备份创建于 {}，大小 {}。",
            format_restore_timestamp(precheck.backup_created_at),
            format_restore_bytes(precheck.backup_size_bytes)
        ),
        format!(
            "当前数据库后端：{}；目标备份类型：{}。",
            database_kind_label(precheck.current_database_kind),
            backup_kind_label(&precheck.semantics)
        ),
        format!(
            "当前恢复方式：{}。",
            restore_mode_label(&precheck.semantics)
        ),
        "恢复不是在线热切换；真正的数据替换或 SQL 导入会发生在下一次服务启动前。".to_string(),
        format!(
            "当前存储配置：{}。",
            summarize_restore_storage(&precheck.current_storage)
        ),
        format!(
            "备份内存储配置：{}。",
            summarize_restore_storage(&precheck.backup_storage)
        ),
    ];
    if let Some(anchor) = precheck.object_rollback_anchor.as_ref() {
        consequences.push(summarize_object_rollback_anchor(anchor));
    }
    consequences.extend(precheck.warnings.iter().cloned());

    let database_label = database_kind_label(precheck.backup_database_kind);
    ConfirmationPlan {
        title: format!("写入 {database_label} 恢复计划"),
        summary: format!(
            "你正在安排在下一次重启时，用备份 {} 恢复当前 {}。执行后当前登录会话会全部失效，需要重新登录。",
            precheck.filename, database_label
        ),
        consequences,
        confirm_label: "写入恢复计划".to_string(),
        cancel_label: "取消".to_string(),
        tone: ConfirmationTone::Danger,
        confirm_phrase: Some(precheck.filename.clone()),
        confirm_hint: Some(format!("请输入 {} 以确认写入恢复计划", precheck.filename)),
    }
}

pub(crate) fn restore_precheck_error_message(precheck: &BackupRestorePrecheckResponse) -> String {
    if precheck.blockers.is_empty() {
        format!("备份 {} 当前不能恢复，请检查服务状态。", precheck.filename)
    } else {
        format!(
            "备份 {} 当前不能恢复：{}",
            precheck.filename,
            precheck.blockers.join("；")
        )
    }
}

fn format_restore_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_restore_timestamp(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn summarize_restore_storage(storage: &BackupRestoreStorageSummary) -> String {
    if storage.storage_backend.is_s3() {
        let bucket = storage
            .s3_bucket
            .clone()
            .unwrap_or_else(|| "未配置 bucket".to_string());
        let endpoint = storage
            .s3_endpoint
            .clone()
            .unwrap_or_else(|| "未配置 endpoint".to_string());
        format!("对象存储 · {} · {}", bucket, endpoint)
    } else {
        format!("本地目录 · {}", storage.local_storage_path)
    }
}

fn database_kind_label(kind: BackupDatabaseFamily) -> &'static str {
    kind.label()
}

fn backup_kind_label(semantics: &BackupSemantics) -> &'static str {
    semantics.kind_label()
}

fn restore_mode_label(semantics: &BackupSemantics) -> &'static str {
    semantics.restore_mode_label()
}

fn summarize_object_rollback_anchor(anchor: &BackupObjectRollbackAnchor) -> String {
    match anchor.strategy {
        BackupObjectRollbackStrategy::LocalDirectorySnapshot => {
            let path = anchor
                .local_storage_path
                .clone()
                .unwrap_or_else(|| "未记录目录".to_string());
            format!(
                "文件回滚锚点：{} @ {}",
                path,
                format_restore_timestamp(anchor.checkpoint_at)
            )
        }
        BackupObjectRollbackStrategy::S3VersionedRollbackAnchor => {
            let bucket = anchor
                .s3_bucket
                .clone()
                .unwrap_or_else(|| "未配置 bucket".to_string());
            let prefix = anchor.s3_prefix.clone().unwrap_or_else(|| "/".to_string());
            let status = anchor
                .s3_bucket_versioning_status
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                "对象回滚锚点：bucket {} · prefix {} · {} · versioning {}",
                bucket,
                prefix,
                format_restore_timestamp(anchor.checkpoint_at),
                status
            )
        }
        BackupObjectRollbackStrategy::Unknown => {
            format!(
                "回滚锚点：未知策略 @ {}",
                format_restore_timestamp(anchor.checkpoint_at)
            )
        }
    }
}
