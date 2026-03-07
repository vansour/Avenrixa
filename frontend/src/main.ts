import './app.css'
import { mount } from 'svelte'
import App from './App.svelte'
import { initTheme } from './stores/theme'

// 初始化主题
initTheme()

// 挂载 Svelte 应用到 #app 元素
const app = mount(App, {
  target: document.getElementById('app')!
})

export default app
