/**
 * 应用常量配置
 * 统一管理所有魔法数字和配置项
 * 支持环境变量覆盖和配置验证
 */

/**
 * 配置验证错误
 */
export class ConfigError extends Error {
  constructor(message: string, public key?: string) {
    super(message)
    this.name = 'ConfigError'
  }
}

/**
 * 配置变更监听器
 */
type ConfigChangeListener = (key: string, oldValue: any, newValue: any) => void

/**
 * 配置管理器
 */
class ConfigManager {
  private listeners: Map<string, Set<ConfigChangeListener>> = new Map()
  private overrides: Map<string, any> = new Map()

  /** 监听配置变更 */
  on(key: string, listener: ConfigChangeListener): () => void {
    if (!this.listeners.has(key)) {
      this.listeners.set(key, new Set())
    }
    this.listeners.get(key)!.add(listener)

    return () => {
      this.listeners.get(key)?.delete(listener)
    }
  }

  /** 触发配置变更事件 */
  emit(key: string, oldValue: any, newValue: any): void {
    const listeners = this.listeners.get(key)
    if (listeners) {
      listeners.forEach(listener => {
        try {
          listener(key, oldValue, newValue)
        } catch (error) {
          console.error(`配置监听器错误 [${key}]:`, error)
        }
      })
    }
  }

  /** 设置配置覆盖值 */
  setOverride(key: string, value: any): void {
    const oldValue = this.overrides.get(key)
    this.overrides.set(key, value)
    this.emit(key, oldValue, value)
  }

  /** 获取配置值（优先使用覆盖值） */
  get(key: string, defaultValue: any): any {
    return this.overrides.get(key) ?? defaultValue
  }

  /** 清除配置覆盖 */
  clearOverride(key: string): void {
    const oldValue = this.overrides.get(key)
    this.overrides.delete(key)
    if (oldValue !== undefined) {
      this.emit(key, oldValue, this.get(key, null))
    }
  }

  /** 清除所有配置覆盖 */
  clearAllOverrides(): void {
    this.overrides.forEach((value, key) => {
      this.emit(key, value, undefined)
    })
    this.overrides.clear()
  }
}

// 创建全局配置管理器
const configManager = new ConfigManager()

/**
 * 从环境变量读取配置
 */
function getEnvOverride<T>(key: string, defaultValue: T): T {
  try {
    const envKey = `VITE_${key.toUpperCase()}`
    const envValue = import.meta.env[envKey]
    if (envValue !== undefined) {
      return envValue as T
    }
  } catch {
    // 忽略环境变量读取错误
  }
  return defaultValue
}

/**
 * 数值范围验证
 */
function validateRange(
  value: number,
  min: number,
  max: number,
  key: string
): void {
  if (value < min || value > max) {
    throw new ConfigError(
      `配置 ${key} 的值 ${value} 不在有效范围 [${min}, ${max}] 内`,
      key
    )
  }
}

/**
 * 验证所有配置
 */
export function validateConfig(): ConfigError[] {
  const errors: ConfigError[] = []

  // 验证文件大小配置
  try {
    validateRange(FILE_SIZE.MAX_UPLOAD_MB, 1, 1024, 'FILE_SIZE.MAX_UPLOAD_MB')
  } catch (e) {
    errors.push(e as ConfigError)
  }

  // 验证分页配置
  try {
    validateRange(PAGINATION.DEFAULT_PAGE_SIZE, 1, 1000, 'PAGINATION.DEFAULT_PAGE_SIZE')
  } catch (e) {
    errors.push(e as ConfigError)
  }

  // 验证 API 超时
  try {
    validateRange(API.TIMEOUT, 1000, 300000, 'API.TIMEOUT')
  } catch (e) {
    errors.push(e as ConfigError)
  }

  return errors
}

/**
 * 导出配置工具
 */
export const configUtils = {
  /** 获取配置值（支持覆盖） */
  get<T>(key: string, defaultValue: T): T {
    return configManager.get(key, defaultValue)
  },

  /** 设置配置覆盖 */
  set(key: string, value: any): void {
    configManager.setOverride(key, value)
  },

  /** 监听配置变更 */
  on(key: string, listener: ConfigChangeListener): () => void {
    return configManager.on(key, listener)
  },

  /** 清除配置覆盖 */
  reset(key?: string): void {
    if (key) {
      configManager.clearOverride(key)
    } else {
      configManager.clearAllOverrides()
    }
  },

  /** 导出所有配置 */
  export(): Record<string, any> {
    return {
      FILE_SIZE,
      FILE_SIZE_PRECISION,
      PAGINATION,
      IMAGE,
      UPLOAD,
      FILTER,
      TAGS,
      VALIDATION,
      THEME,
      API,
      TOAST,
      ANIMATION,
      DEBOUNCE,
      VIRTUAL_SCROLL,
      KEYBOARD,
      STORAGE_KEYS,
      PERFORMANCE,
      A11Y,
      REGEX,
      COLORS,
      NETWORK_CHECK,
      EDITOR_DEFAULTS,
      FORM_DEFAULTS
    }
  }
}

/** 文件大小常量 */
export const FILE_SIZE = {
  B: 1,
  KB: 1024,
  MB: 1024 * 1024,
  GB: 1024 * 1024 * 1024,
  TB: 1024 * 1024 * 1024 * 1024,
  MAX_UPLOAD_MB: getEnvOverride('MAX_UPLOAD_MB', 10),
  MAX_UPLOAD_BYTES: getEnvOverride('MAX_UPLOAD_MB', 10) * 1024 * 1024,
} as const

/** 文件大小显示精度 */
export const FILE_SIZE_PRECISION = {
  B: 0,
  KB: 1,
  MB: 1,
  GB: 2,
  TB: 2,
} as const

/** 分页配置 */
export const PAGINATION = {
  DEFAULT_PAGE_SIZE: getEnvOverride('DEFAULT_PAGE_SIZE', 20),
  MAX_PAGE_SIZE: getEnvOverride('MAX_PAGE_SIZE', 100),
  ADMIN_PAGE_SIZE: getEnvOverride('ADMIN_PAGE_SIZE', 50),
} as const

/** 图片配置 */
export const IMAGE = {
  // 缩略图尺寸
  THUMBNAIL_WIDTH: getEnvOverride('THUMBNAIL_WIDTH', 300),
  THUMBNAIL_HEIGHT: getEnvOverride('THUMBNAIL_HEIGHT', 200),

  // 懒加载配置
  LAZY_LOAD_THRESHOLD: getEnvOverride('LAZY_LOAD_THRESHOLD', 50),
  LAZY_LOAD_ROOT_MARGIN: getEnvOverride('LAZY_LOAD_ROOT_MARGIN', '200px'),
  LAZY_LOAD_ROOT_MARGIN_NUM: 200,
  PREFETCH_COUNT: getEnvOverride('PREFETCH_COUNT', 3),

  // 图片质量
  DEFAULT_QUALITY: getEnvOverride('DEFAULT_QUALITY', 0.85),
  JPEG_QUALITY: getEnvOverride('JPEG_QUALITY', 0.85),
  PNG_QUALITY: getEnvOverride('PNG_QUALITY', 0.9),
  WEBP_QUALITY: getEnvOverride('WEBP_QUALITY', 0.8),

  // 支持的图片格式
  SUPPORTED_FORMATS: getEnvOverride('SUPPORTED_FORMATS', ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg']) as readonly string[],

  // 最大上传数量
  MAX_UPLOAD_FILES: getEnvOverride('MAX_UPLOAD_FILES', 100),

  // 默认宽高比
  DEFAULT_ASPECT_RATIO: getEnvOverride('DEFAULT_ASPECT_RATIO', '4/3'),
} as const

/** 上传配置 */
export const UPLOAD = {
  MAX_FILES_PER_REQUEST: getEnvOverride('MAX_FILES_PER_REQUEST', 10),
  RETRY_COUNT: getEnvOverride('UPLOAD_RETRY_COUNT', 3),
  RETRY_DELAY_MS: getEnvOverride('UPLOAD_RETRY_DELAY_MS', 1000),
  TIMEOUT_MS: getEnvOverride('UPLOAD_TIMEOUT_MS', 60000),
  PROGRESS_UPDATE_INTERVAL: getEnvOverride('PROGRESS_UPDATE_INTERVAL', 100),
} as const

/** 筛选配置 */
export const FILTER = {
  // 亮度、对比度、饱和度范围
  MIN_BRIGHTNESS: 0,
  MAX_BRIGHTNESS: 255,
  DEFAULT_BRIGHTNESS: 128,

  MIN_CONTRAST: 0,
  MAX_CONTRAST: 255,
  DEFAULT_CONTRAST: 128,

  MIN_SATURATION: 0,
  MAX_SATURATION: 255,
  DEFAULT_SATURATION: 128,

  // 透明度范围
  MIN_OPACITY: 0,
  MAX_OPACITY: 255,
  DEFAULT_OPACITY: 128,

  // 旋转角度
  ROTATE_ANGLES: [-90, 90, 180] as const,

  // 水印位置
  WATERMARK_POSITIONS: [
    'top-left',
    'top-right',
    'bottom-left',
    'bottom-right',
  ] as const,
} as const

/** 标签配置 */
export const TAGS = {
  MAX_TAGS_PER_IMAGE: getEnvOverride('MAX_TAGS_PER_IMAGE', 10),
  MAX_TAG_LENGTH: getEnvOverride('MAX_TAG_LENGTH', 20),
  MIN_TAG_LENGTH: getEnvOverride('MIN_TAG_LENGTH', 1),
  SEPARATOR: ',',
} as const

/** 验证规则 */
export const VALIDATION = {
  // 用户名
  USERNAME_MIN_LENGTH: getEnvOverride('USERNAME_MIN_LENGTH', 3),
  USERNAME_MAX_LENGTH: getEnvOverride('USERNAME_MAX_LENGTH', 50),

  // 密码
  PASSWORD_MIN_LENGTH: getEnvOverride('PASSWORD_MIN_LENGTH', 6),
  PASSWORD_MAX_LENGTH: getEnvOverride('PASSWORD_MAX_LENGTH', 128),

  // 重命名
  FILENAME_MAX_LENGTH: getEnvOverride('FILENAME_MAX_LENGTH', 255),
} as const

/** API 配置 */
export const API = {
  BASE_URL: getEnvOverride('API_BASE_URL', '/api'),
  TIMEOUT: getEnvOverride('API_TIMEOUT', 30000),
  MAX_RETRIES: getEnvOverride('API_MAX_RETRIES', 3),
  RETRY_DELAY: getEnvOverride('API_RETRY_DELAY', 1000),
} as const

/** Toast 配置 */
export const TOAST = {
  DEFAULT_DURATION: getEnvOverride('TOAST_DEFAULT_DURATION', 3000),
  SUCCESS_DURATION: getEnvOverride('TOAST_SUCCESS_DURATION', 3000),
  ERROR_DURATION: getEnvOverride('TOAST_ERROR_DURATION', 5000),
  INFO_DURATION: getEnvOverride('TOAST_INFO_DURATION', 3000),
  WARNING_DURATION: getEnvOverride('TOAST_WARNING_DURATION', 4000),

  MAX_COUNT: getEnvOverride('TOAST_MAX_COUNT', 3),
  PRIORITY: {
    LOW: 'low',
    NORMAL: 'normal',
    HIGH: 'high',
  } as const,
} as const

/** 动画配置 */
export const ANIMATION = {
  FAST: 150,
  NORMAL: 300,
  SLOW: 500,

  TRANSITION_FAST: '0.15s',
  TRANSITION_NORMAL: '0.3s',
  TRANSITION_SLOW: '0.5s',
} as const

/** 防抖/节流延迟 */
export const DEBOUNCE = {
  DEFAULT: getEnvOverride('DEBOUNCE_DEFAULT', 300),
  SEARCH: getEnvOverride('DEBOUNCE_SEARCH', 300),
  SCROLL: 16,
  RESIZE: getEnvOverride('DEBOUNCE_RESIZE', 100),
  INPUT: getEnvOverride('DEBOUNCE_INPUT', 300),
} as const

/** 虚拟滚动配置 */
export const VIRTUAL_SCROLL = {
  DEFAULT_BUFFER: getEnvOverride('VIRTUAL_SCROLL_DEFAULT_BUFFER', 5),
  LOW_END_BUFFER: getEnvOverride('VIRTUAL_SCROLL_LOW_END_BUFFER', 3),
  ITEM_HEIGHT: getEnvOverride('VIRTUAL_SCROLL_ITEM_HEIGHT', 280),
} as const

/** 键盘快捷键 */
export const KEYBOARD = {
  SELECT_ALL: { ctrl: true, meta: true, key: 'a' },
  COPY: { ctrl: true, meta: true, key: 'c' },
  PASTE: { ctrl: true, meta: true, key: 'v' },
  ESCAPE: 'Escape',
  DELETE: 'Delete',
  ENTER: 'Enter',
  SPACE: ' ',
  ARROW_UP: 'ArrowUp',
  ARROW_DOWN: 'ArrowDown',
  ARROW_LEFT: 'ArrowLeft',
  ARROW_RIGHT: 'ArrowRight',
} as const

/** 存储键 */
export const STORAGE_KEYS = {
  AUTH: getEnvOverride('STORAGE_AUTH', 'vansour_auth'),
  VIRTUAL_SCROLL: getEnvOverride('STORAGE_VIRTUAL_SCROLL', 'virtualScroll'),
  LAST_VISIT: getEnvOverride('STORAGE_LAST_VISIT', 'lastVisit'),
  USER_PREFERENCES: getEnvOverride('STORAGE_USER_PREFERENCES', 'userPreferences'),
} as const

/** 性能配置 */
export const PERFORMANCE = {
  LOW_END_CORES: 2,
  CACHE_TTL: getEnvOverride('PERFORMANCE_CACHE_TTL', 5 * 60 * 1000),
  LOW_END_CACHE_TTL: getEnvOverride('PERFORMANCE_LOW_END_CACHE_TTL', 10 * 60 * 1000),
  BATCH_SIZE: getEnvOverride('PERFORMANCE_BATCH_SIZE', 10),
  LOW_END_BATCH_SIZE: getEnvOverride('PERFORMANCE_LOW_END_BATCH_SIZE', 5),
  LAZY_THRESHOLD: getEnvOverride('PERFORMANCE_LAZY_THRESHOLD', 50),
  LOW_END_LAZY_THRESHOLD: getEnvOverride('PERFORMANCE_LOW_END_LAZY_THRESHOLD', 200),
} as const

/** 可访问性 */
export const A11Y = {
  IMAGE_LIST_REGION: '图片列表',
  IMAGE_ITEM_LABEL_PREFIX: '图片',
  BULK_ACTIONS_GROUP: '批量操作',
  IMAGE_COUNT_LABEL: '图片数量',

  FOCUS_SELECTOR: '[tabindex="0"]',
  SKIP_LINK_SELECTOR: '.skip-link',
} as const

/** 正则表达式 */
export const REGEX = {
  EMAIL: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
  USERNAME: /^[a-zA-Z0-9_]{3,50}$/,
  URL: /^https?:\/\/.+/,
  PHONE: /^1[3-9]\d{9}$/,
  NUMBER: /^\d+$/,
  IMAGE_EXTENSION: /\.(jpe?g|png|gif|webp|bmp|svg)$/i,
} as const

/** 颜色主题 */
export const COLORS = {
  PRIMARY: getEnvOverride('COLOR_PRIMARY', '#007bff'),
  PRIMARY_HOVER: getEnvOverride('COLOR_PRIMARY_HOVER', '#0056b3'),
  SUCCESS: getEnvOverride('COLOR_SUCCESS', '#28a745'),
  SUCCESS_HOVER: getEnvOverride('COLOR_SUCCESS_HOVER', '#218838'),
  DANGER: getEnvOverride('COLOR_DANGER', '#dc3545'),
  DANGER_HOVER: getEnvOverride('COLOR_DANGER_HOVER', '#c82333'),
  WARNING: getEnvOverride('COLOR_WARNING', '#ffc107'),
  WARNING_HOVER: getEnvOverride('COLOR_WARNING_HOVER', '#e0a800'),
  INFO: getEnvOverride('COLOR_INFO', '#17a2b8'),
  INFO_HOVER: getEnvOverride('COLOR_INFO_HOVER', '#138496'),
  SECONDARY: getEnvOverride('COLOR_SECONDARY', '#6c757d'),
  SECONDARY_HOVER: getEnvOverride('COLOR_SECONDARY_HOVER', '#5a6268'),
} as const

/** 网络状态检查间隔 */
export const NETWORK_CHECK = {
  INTERVAL: getEnvOverride('NETWORK_CHECK_INTERVAL', 30000),
  RETRY_DELAY: getEnvOverride('NETWORK_CHECK_RETRY_DELAY', 5000),
} as const

/** 编辑器默认值 */
export const EDITOR_DEFAULTS = {
  rotate: null as number | null,
  brightness: 128,
  contrast: 128,
  saturation: 128,
  grayscale: false,
  sepia: false,
  watermarkText: '',
  watermarkPosition: '',
  watermarkOpacity: 128,
  convertFormat: '',
} as const

/** 表单默认值 */
export const FORM_DEFAULTS = {
  USERNAME_MIN: getEnvOverride('USERNAME_MIN', 3),
  USERNAME_MAX: getEnvOverride('USERNAME_MAX', 50),
  PASSWORD_MIN: getEnvOverride('PASSWORD_MIN', 6),
  PASSWORD_MAX: getEnvOverride('PASSWORD_MAX', 128),
} as const

export const THEME = {
  PRIMARY: '#667eea',
  SECONDARY: '#764ba2',
} as const
