/**
 * 应用常量配置
 * 统一管理所有魔法数字和配置项
 */

/**
 * 文件大小常量
 */
export const FILE_SIZE = {
  B: 1,
  KB: 1024,
  MB: 1024 * 1024,
  GB: 1024 * 1024 * 1024,
  TB: 1024 * 1024 * 1024 * 1024,
  MAX_UPLOAD_MB: 10,
  MAX_UPLOAD_BYTES: 10 * 1024 * 1024,
} as const

/**
 * 文件大小显示精度
 */
export const FILE_SIZE_PRECISION = {
  B: 0,
  KB: 1,
  MB: 1,
  GB: 2,
  TB: 2,
} as const

/**
 * 分页配置
 */
export const PAGINATION = {
  DEFAULT_PAGE_SIZE: 20,
  MAX_PAGE_SIZE: 100,
  ADMIN_PAGE_SIZE: 50,
} as const

/**
 * 图片配置
 */
export const IMAGE = {
  // 缩略图尺寸
  THUMBNAIL_WIDTH: 300,
  THUMBNAIL_HEIGHT: 200,

  // 懒加载配置
  LAZY_LOAD_THRESHOLD: 50,
  LAZY_LOAD_ROOT_MARGIN: '200px',
  PREFETCH_COUNT: 3,

  // 图片质量
  DEFAULT_QUALITY: 0.85,
  JPEG_QUALITY: 0.85,
  PNG_QUALITY: 0.9,
  WEBP_QUALITY: 0.8,

  // 支持的图片格式
  SUPPORTED_FORMATS: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg'],

  // 最大上传数量
  MAX_UPLOAD_FILES: 100,

  // 默认宽高比
  DEFAULT_ASPECT_RATIO: '4/3',
} as const

/**
 * 上传配置
 */
export const UPLOAD = {
  MAX_FILES_PER_REQUEST: 10,
  RETRY_COUNT: 3,
  RETRY_DELAY_MS: 1000,
  TIMEOUT_MS: 60000,
  PROGRESS_UPDATE_INTERVAL: 100,
} as const

/**
 * 筛选配置
 */
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

/**
 * 标签配置
 */
export const TAGS = {
  MAX_TAGS_PER_IMAGE: 10,
  MAX_TAG_LENGTH: 20,
  MIN_TAG_LENGTH: 1,
  SEPARATOR: ',',
} as const

/**
 * 验证规则
 */
export const VALIDATION = {
  // 用户名
  USERNAME_MIN_LENGTH: 3,
  USERNAME_MAX_LENGTH: 50,

  // 密码
  PASSWORD_MIN_LENGTH: 6,
  PASSWORD_MAX_LENGTH: 128,

  // 分类名称
  CATEGORY_MIN_LENGTH: 1,
  CATEGORY_MAX_LENGTH: 50,

  // 重命名
  FILENAME_MAX_LENGTH: 255,
} as const

/**
 * 主题配置
 */
export const THEME = {
  STORAGE_KEY: 'theme',
  DEFAULT: 'light',
  VALUES: ['light', 'dark'] as const,

  // 自动检测系统主题
  AUTO_DETECT: true,
} as const

/**
 * API 配置
 */
export const API = {
  BASE_URL: '/api',
  TIMEOUT: 30000,
  MAX_RETRIES: 3,
  RETRY_DELAY: 1000,
} as const

/**
 * Toast 配置
 */
export const TOAST = {
  DEFAULT_DURATION: 3000,
  SUCCESS_DURATION: 3000,
  ERROR_DURATION: 5000,
  INFO_DURATION: 3000,
  WARNING_DURATION: 4000,

  MAX_COUNT: 3,
  PRIORITY: {
    LOW: 'low',
    NORMAL: 'normal',
    HIGH: 'high',
  } as const,
} as const

/**
 * 动画配置
 */
export const ANIMATION = {
  FAST: 150,
  NORMAL: 300,
  SLOW: 500,

  TRANSITION_FAST: '0.15s',
  TRANSITION_NORMAL: '0.3s',
  TRANSITION_SLOW: '0.5s',
} as const

/**
 * 防抖/节流延迟
 */
export const DEBOUNCE = {
  DEFAULT: 300,
  SEARCH: 500,
  SCROLL: 16, // 60fps
  RESIZE: 100,
  INPUT: 300,
} as const

/**
 * 虚拟滚动配置
 */
export const VIRTUAL_SCROLL = {
  DEFAULT_BUFFER: 5,
  LOW_END_BUFFER: 3,
  ITEM_HEIGHT: 280,
} as const

/**
 * 键盘快捷键
 */
export const KEYBOARD = {
  // 全选
  SELECT_ALL: { ctrl: true, meta: true, key: 'a' },

  // 复制
  COPY: { ctrl: true, meta: true, key: 'c' },

  // 粘贴
  PASTE: { ctrl: true, meta: true, key: 'v' },

  // 取消
  ESCAPE: 'Escape',

  // 删除
  DELETE: 'Delete',

  // 回车
  ENTER: 'Enter',

  // 空格
  SPACE: ' ',

  // 方向键
  ARROW_UP: 'ArrowUp',
  ARROW_DOWN: 'ArrowDown',
  ARROW_LEFT: 'ArrowLeft',
  ARROW_RIGHT: 'ArrowRight',
} as const

/**
 * 存储键
 */
export const STORAGE_KEYS = {
  AUTH: 'vansour_auth',
  THEME: 'theme',
  VIRTUAL_SCROLL: 'virtualScroll',
  LAST_VISIT: 'lastVisit',
  USER_PREFERENCES: 'userPreferences',
} as const

/**
 * 性能配置
 */
export const PERFORMANCE = {
  // 设备检测
  LOW_END_CORES: 2,

  // 缓存 TTL
  CACHE_TTL: 5 * 60 * 1000, // 5分钟
  LOW_END_CACHE_TTL: 10 * 60 * 1000, // 10分钟

  // 批处理大小
  BATCH_SIZE: 10,
  LOW_END_BATCH_SIZE: 5,

  // 懒加载阈值
  LAZY_THRESHOLD: 50,
  LOW_END_LAZY_THRESHOLD: 200,
} as const

/**
 * 可访问性
 */
export const A11Y = {
  // ARIA 标签
  IMAGE_LIST_REGION: '图片列表',
  IMAGE_ITEM_LABEL_PREFIX: '图片',
  BULK_ACTIONS_GROUP: '批量操作',
  IMAGE_COUNT_LABEL: '图片数量',

  // 焦点管理
  FOCUS_SELECTOR: '[tabindex="0"]',
  SKIP_LINK_SELECTOR: '.skip-link',
} as const

/**
 * 正则表达式
 */
export const REGEX = {
  EMAIL: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
  USERNAME: /^[a-zA-Z0-9_]{3,50}$/,
  URL: /^https?:\/\/.+/,
  PHONE: /^1[3-9]\d{9}$/,
  NUMBER: /^\d+$/,
  IMAGE_EXTENSION: /\.(jpe?g|png|gif|webp|bmp|svg)$/i,
} as const

/**
 * 颜色主题
 */
export const COLORS = {
  PRIMARY: '#007bff',
  PRIMARY_HOVER: '#0056b3',
  SUCCESS: '#28a745',
  SUCCESS_HOVER: '#218838',
  DANGER: '#dc3545',
  DANGER_HOVER: '#c82333',
  WARNING: '#ffc107',
  WARNING_HOVER: '#e0a800',
  INFO: '#17a2b8',
  INFO_HOVER: '#138496',
  SECONDARY: '#6c757d',
  SECONDARY_HOVER: '#5a6268',
} as const

/**
 * 网络状态检查间隔
 */
export const NETWORK_CHECK = {
  INTERVAL: 30000, // 30秒
  RETRY_DELAY: 5000,
} as const

/**
 * 编辑器默认值
 */
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

/**
 * 表单默认值
 */
export const FORM_DEFAULTS = {
  USERNAME_MIN: 3,
  USERNAME_MAX: 50,
  PASSWORD_MIN: 6,
  PASSWORD_MAX: 128,
  CATEGORY_NAME_MAX: 50,
} as const
