# VanSour Image (Svelte Version)

VanSour Image 是一个简单快速的图片托管服务，使用 Svelte 5 + Vite 7 + Rust 后端。

## 技术栈

### 前端
- **Svelte 5.21** - 响应式 UI 框架（无虚拟 DOM）
- **Vite 7.3** - 构建工具
- **TypeScript 5.9** - 类型安全
- **Tailwind CSS 4.2** - 样式框架
- **svelte-i18n 4.0** - 国际化
- **Lucide Svelte 0.577** - 图标库

### 后端
- **Rust** + **Axum** - Web 框架
- **PostgreSQL** - 数据库
- **Redis** - 缓存

## 功能特性

- 🔐 用户认证（注册/登录/登出）
- 📤 图片上传（拖拽/点击选择）
- 🖼️ 图片编辑（旋转/滤镜/水印）
- 🔍 搜索和筛选
- 📋 分页浏览（支持游标分页）
- 🗂️ 回收站（软删除/恢复）
- ⚙️ 多语言支持（中文/英文）
- 🌓 Glass morphism UI 设计
- 📱 响应式设计
- ⌨️ 键盘快捷键支持
- 🌐 离线状态检测
- ♿ 无障碍支持 (A11Y)

## 开发

```bash
# 安装依赖
cd frontend-svelte
npm install

# 开发服务器
npm run dev

# 构建生产版本
npm run build

# 预览构建结果
npm run preview

# 运行测试
npm test

# 测试 UI
npm run test:ui
```

## 目录结构

```
frontend-svelte/
├── src/
│   ├── components/      # Svelte 组件
│   │   ├── App.svelte
│   │   ├── Router.svelte
│   │   ├── Auth.svelte
│   │   ├── Home.svelte
│   │   ├── Settings.svelte
│   │   ├── Profile.svelte
│   │   ├── Trash.svelte
│   │   ├── ImageCard.svelte
│   │   ├── ImageEditor.svelte
│   │   ├── ImagePreview.svelte
│   │   ├── ImageList.svelte
│   │   ├── UploadZone.svelte
│   │   ├── UserMenu.svelte
│   │   ├── Toast.svelte
│   │   ├── Dialog.svelte
│   │   ├── Modal.svelte
│   │   ├── LazyImage.svelte
│   │   ├── Skeleton.svelte
│   │   ├── LocaleSwitcher.svelte
│   │   ├── Button.svelte
│   │   ├── Input.svelte
│   │   ├── Select.svelte
│   │   ├── Badge.svelte
│   │   ├── Toggle.svelte
│   │   └── Avatar.svelte
│   ├── stores/         # Svelte stores 状态管理
│   │   ├── auth.ts
│   │   ├── toast.ts
│   │   ├── dialog.ts
│   │   ├── images.ts
│   │   ├── api.ts
│   │   ├── routes.ts
│   │   └── router.ts
│   ├── composables/    # 组合式函数
│   │   ├── useKeyboard.ts
│   │   └── useNetwork.ts
│   ├── utils/          # 工具函数
│   │   ├── format.ts
│   │   ├── api.ts
│   │   ├── validation.ts
│   │   ├── debounce.ts
│   │   └── clipboard.ts
│   ├── types/          # TypeScript 类型
│   │   └── index.ts
│   ├── constants/      # 常量配置
│   │   └── index.ts
│   ├── app.css         # 全局样式
│   ├── main.ts         # 入口文件
│   ├── components/      # 图标导出
│   └── vite-env.d.ts   # 环境类型
├── index.html          # HTML 模板
├── index.dev.html      # 开发环境模板
├── vite.config.ts     # Vite 配置
├── tailwind.config.js # Tailwind 配置
├── tsconfig.json      # TypeScript 配置
├── svelte.config.js   # Svelte 配置
├── postcss.config.js  # PostCSS 配置
├── .prettierrc       # Prettier 配置
└── package.json       # 依赖配置
```

## 从 Vue 迁移的主要差异

### 组件语法
- Vue 的 `v-if/v-else/v-for` → Svelte 的 `{#if}/{:else}/{/each}`
- Vue 的 `v-model` → Svelte 的 `bind:value`
- Vue 的 `@click` → Svelte 的 `on:click`
- Vue 的 `:class` → Svelte 的 `class:`

### 状态管理
- Vue 的 Pinia stores → Svelte 的 writable/derived stores
- 不再需要 Pinia，使用原生 Svelte stores
- `computed` → Svelte 的响应式语句 `$:`

### 路由
- Vue Router → 自定义的哈希路由
- Router.svelte 组件处理路由逻辑

### API 请求
- 保持相同的后端 API 接口
- 使用 fetch API 替代 axios
- 自定义 api.ts 工具函数

### 图标
- lucide-vue-next → lucide-svelte
- 统一从 components/icons.ts 导入

### 国际化
- vue-i18n → svelte-i18n
- 使用 Svelte 的 i18n 插件

## 性能优化

- Svelte 编译为原生 JavaScript，无虚拟 DOM
- 组件零运行时开销
- 细粒度响应式更新
- IntersectionObserver 实现懒加载
- 防抖/节流优化频繁操作
- 虚拟滚动支持大列表
- CSS 动画替代 JavaScript 动画

## 已完成的功能

✅ 用户认证系统
✅ 图片上传（拖拽/点击）
✅ 图片列表（分页/搜索/排序）
✅ 图片编辑器（旋转/滤镜/水印）
✅ 回收站（软删除/恢复/永久删除）
✅ 个人资料（修改密码）
✅ 系统设置（多配置项）
✅ 用户管理（管理员）
✅ 审计日志
✅ 数据库备份
✅ Toast 通知
✅ 对话框确认
✅ 模态框
✅ 懒加载图片
✅ 骨架屏加载
✅ 键盘快捷键
✅ 网络状态检测
✅ 国际化支持
✅ 响应式布局
✅ 可访问性支持

## 组件列表

### 页面组件
- `App.svelte` - 根组件
- `Router.svelte` - 路由组件
- `Auth.svelte` - 认证页面
- `Home.svelte` - 首页
- `Settings.svelte` - 设置页面
- `Profile.svelte` - 个人资料页面
- `Trash.svelte` - 回收站页面

### UI 组件
- `ImageCard.svelte` - 图片卡片
- `ImageEditor.svelte` - 图片编辑器
- `ImagePreview.svelte` - 图片预览
- `ImageList.svelte` - 图片列表
- `UploadZone.svelte` - 上传区域
- `UserMenu.svelte` - 用户菜单
- `Toast.svelte` - 通知组件
- `Dialog.svelte` - 对话框
- `Modal.svelte` - 模态框
- `LazyImage.svelte` - 懒加载图片
- `Skeleton.svelte` - 骨架屏
- `LocaleSwitcher.svelte` - 语言切换
- `Button.svelte` - 按钮组件
- `Input.svelte` - 输入框组件
- `Select.svelte` - 选择框组件
- `Badge.svelte` - 徽章组件
- `Toggle.svelte` - 开关组件
- `Avatar.svelte` - 头像组件

### Composables
- `useKeyboard.ts` - 键盘快捷键
- `useNetwork.ts` - 网络状态检测

### Stores
- `auth.ts` - 认证状态
- `toast.ts` - 通知状态
- `dialog.ts` - 对话框状态
- `images.ts` - 图片状态
- `api.ts` - API 调用
- `routes.ts` - 路由配置
- `router.ts` - 密码修改状态

## 待测试功能

- [ ] 完整的集成测试
- [ ] E2E 测试
- [ ] 性能测试
- [ ] 浏览器兼容性测试

## License

MIT
