import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "./style.css";
import { useTheme } from "./composables/useTheme";
import { usePlugins } from "./composables/usePlugins";

const app = createApp(App);
app.use(router);

// Load theme (applies CSS vars for branding)
useTheme().load();

// Load plugin pages (registers dynamic routes)
router.isReady().then(() => {
  usePlugins().load(router);
});

app.mount("#app");
