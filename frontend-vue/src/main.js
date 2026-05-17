import { createApp } from 'vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import App from './App.vue'
import MobileApp from './MobileApp.vue'

const isMobileEntry = import.meta.env.VITE_APP_ENTRY === 'mobile'
  || window.location.port === '18101'
  || window.location.pathname.startsWith('/mobile')

const entry = isMobileEntry ? MobileApp : App
const app = createApp(entry)

app.use(ElementPlus)
app.mount('#app')
