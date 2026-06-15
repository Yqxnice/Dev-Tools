import { createApp } from "vue"
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import naive from 'naive-ui'
import App from "./App.vue"
import './theme/global.css'

const app = createApp(App)
const pinia = createPinia()
pinia.use(piniaPluginPersistedstate)
app.use(pinia)
app.use(naive)
app.mount("#app")
