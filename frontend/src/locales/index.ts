import { createI18n } from 'vue-i18n'
import type { Locale } from 'vue-i18n'

// 中文翻译
const zhCN = {
  common: {
    app: {
      name: 'VanSour Image',
      tagline: '简单快速的图片托管服务'
    },
    actions: {
      upload: '上传',
      save: '保存',
      cancel: '取消',
      delete: '删除',
      edit: '编辑',
      rename: '重命名',
      duplicate: '复制',
      preview: '预览',
      copy: '复制链接',
      confirm: '确认',
      close: '关闭'
    },
    status: {
      loading: '加载中...',
      success: '成功',
      error: '错误',
      warning: '警告'
    }
  },
  auth: {
    login: {
      title: '登录',
      username: '用户名',
      password: '密码',
      submit: '登录',
      noAccount: '还没有账号？',
      registerHere: '立即注册',
      success: '登录成功'
    },
    register: {
      title: '注册',
      username: '用户名',
      password: '密码',
      confirmPassword: '确认密码',
      submit: '注册',
      hasAccount: '已有账号？',
      loginHere: '立即登录',
      success: '注册成功'
    },
    logout: '退出登录'
  },
  images: {
    list: {
      title: '我的图片',
      uploadHere: '拖拽图片到这里或点击上传',
      uploadButton: '选择图片',
      searchPlaceholder: '搜索图片...',
      empty: '暂无图片',
      total: '共 {count} 张图片',
      size: '大小',
      views: '浏览'
    },
    upload: {
      title: '上传图片',
      maxSize: '最大文件大小: {maxSize}',
      allowedTypes: '支持格式: {types}',
      uploading: '上传中...',
      success: '上传成功',
      failure: '上传失败'
    },
    actions: {
      rename: '重命名',
      setExpiry: '设置过期时间',
      delete: '删除',
      duplicate: '复制图片',
      edit: '编辑图片',
      copyLink: '复制链接'
    },
    delete: {
      confirm: '确定要删除这张图片吗？',
      multiple: '确定要删除 {count} 张图片吗？'
    }
  },
  categories: {
    all: '全部图片',
    uncategorized: '未分类',
    create: '创建分类',
    name: '分类名称',
    deleteConfirm: '确定要删除此分类吗？',
    empty: '暂无分类'
  },
  settings: {
    title: '系统设置',
    sections: {
      basic: '基本设置',
      upload: '上传设置',
      storage: '存储设置',
      security: '安全设置',
      system: '系统信息'
    },
    basic: {
      siteName: '站点名称',
      siteDescription: '站点描述'
    },
    upload: {
      maxSize: '最大上传大小 (MB)',
      dailyLimit: '每日上传限制 (0=无限制)',
      allowedExtensions: '允许的扩展名'
    },
    storage: {
      retentionDays: '删除保留天数',
      cleanupInterval: '清理间隔',
      autoCleanup: '自动清理',
      intervalValues: {
        daily: '每天',
        weekly: '每周',
        monthly: '每月'
      }
    },
    security: {
      requireApproval: '需要审核',
      enableRegistration: '允许注册'
    },
    save: '保存设置',
    saved: '设置已保存',
    saveFailed: '保存失败'
  },
  profile: {
    title: '个人资料',
    changePassword: '修改密码',
    currentPassword: '当前密码',
    newPassword: '新密码',
    confirmPassword: '确认新密码',
    submit: '修改密码',
    success: '密码修改成功',
    failure: '密码修改失败'
  },
  admin: {
    title: '管理面板',
    stats: {
      users: '用户数',
      images: '图片数',
      storage: '存储空间',
      views: '总浏览量',
      diskUsage: '磁盘使用率'
    },
    actions: {
      backup: '备份数据库',
      auditLogs: '审计日志',
      userManagement: '用户管理'
    }
  },
  trash: {
    title: '回收站',
    items: '回收站项目',
    empty: '回收站为空',
    restore: '恢复',
    deletePermanently: '永久删除',
    restoreMultiple: '恢复选中 ({count})',
    deleteMultiple: '永久删除选中 ({count})'
  },
  a11y: {
    imageItemPrefix: '图片',
    loadingImage: '加载图片',
    errorImage: '图片加载失败'
  }
}

// 英文翻译
const enUS: Locale = {
  common: {
    app: {
      name: 'VanSour Image',
      tagline: 'Simple and Fast Image Hosting'
    },
    actions: {
      upload: 'Upload',
      save: 'Save',
      cancel: 'Cancel',
      delete: 'Delete',
      edit: 'Edit',
      rename: 'Rename',
      duplicate: 'Duplicate',
      preview: 'Preview',
      copy: 'Copy Link',
      confirm: 'Confirm',
      close: 'Close'
    },
    status: {
      loading: 'Loading...',
      success: 'Success',
      error: 'Error',
      warning: 'Warning'
    }
  },
  auth: {
    login: {
      title: 'Login',
      username: 'Username',
      password: 'Password',
      submit: 'Login',
      noAccount: "Don't have an account?",
      registerHere: 'Register here',
      success: 'Login successful'
    },
    register: {
      title: 'Register',
      username: 'Username',
      password: 'Password',
      confirmPassword: 'Confirm Password',
      submit: 'Register',
      hasAccount: 'Already have an account?',
      loginHere: 'Login here',
      success: 'Registration successful'
    },
    logout: 'Logout'
  },
  images: {
    list: {
      title: 'My Images',
      uploadHere: 'Drag images here or click to upload',
      uploadButton: 'Select Images',
      searchPlaceholder: 'Search images...',
      empty: 'No images yet',
      total: 'Total {count} images',
      size: 'Size',
      views: 'Views'
    },
    upload: {
      title: 'Upload Images',
      maxSize: 'Max file size: {maxSize}',
      allowedTypes: 'Supported formats: {types}',
      uploading: 'Uploading...',
      success: 'Upload successful',
      failure: 'Upload failed'
    },
    actions: {
      rename: 'Rename',
      setExpiry: 'Set Expiry',
      delete: 'Delete',
      duplicate: 'Duplicate',
      edit: 'Edit',
      copyLink: 'Copy Link'
    },
    delete: {
      confirm: 'Are you sure you want to delete this image?',
      multiple: 'Are you sure you want to delete {count} images?'
    }
  },
  categories: {
    all: 'All Images',
    uncategorized: 'Uncategorized',
    create: 'Create Category',
    name: 'Category Name',
    deleteConfirm: 'Delete this category?',
    empty: 'No categories'
  },
  settings: {
    title: 'Settings',
    sections: {
      basic: 'Basic Settings',
      upload: 'Upload Settings',
      storage: 'Storage Settings',
      security: 'Security Settings',
      system: 'System Info'
    },
    basic: {
      siteName: 'Site Name',
      siteDescription: 'Site Description'
    },
    upload: {
      maxSize: 'Max Upload Size (MB)',
      dailyLimit: 'Daily Upload Limit (0=unlimited)',
      allowedExtensions: 'Allowed Extensions'
    },
    storage: {
      retentionDays: 'Deleted Images Retention (days)',
      cleanupInterval: 'Cleanup Interval',
      autoCleanup: 'Auto Cleanup',
      intervalValues: {
        daily: 'Daily',
        weekly: 'Weekly',
        monthly: 'Monthly'
      }
    },
    security: {
      requireApproval: 'Require Approval',
      enableRegistration: 'Enable Registration'
    },
    save: 'Save Settings',
    saved: 'Settings saved',
    saveFailed: 'Save failed'
  },
  profile: {
    title: 'Profile',
    changePassword: 'Change Password',
    currentPassword: 'Current Password',
    newPassword: 'New Password',
    confirmPassword: 'Confirm New Password',
    submit: 'Change Password',
    success: 'Password changed successfully',
    failure: 'Failed to change password'
  },
  admin: {
    title: 'Admin Panel',
    stats: {
      users: 'Users',
      images: 'Images',
      storage: 'Storage',
      views: 'Total Views',
      diskUsage: 'Disk Usage'
    },
    actions: {
      backup: 'Backup Database',
      auditLogs: 'Audit Logs',
      userManagement: 'User Management'
    }
  },
  trash: {
    title: 'Trash',
    items: 'Trash Items',
    empty: 'Trash is empty',
    restore: 'Restore',
    deletePermanently: 'Delete Permanently',
    restoreMultiple: 'Restore ({count})',
    deleteMultiple: 'Delete Permanently ({count})'
  },
  a11y: {
    imageItemPrefix: 'Image',
    loadingImage: 'Loading image',
    errorImage: 'Failed to load image'
  }
}

const messages = {
  'zh-CN': zhCN,
  'en-US': enUS
}

export type MessageSchema = keyof typeof messages

const i18n = createI18n<[MessageSchema]>({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'en-US',
  messages
})

export { i18n, zhCN, enUS }
export default i18n
