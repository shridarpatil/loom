import { ref } from "vue";
import type { Router } from "vue-router";

export interface PluginPage {
  route: string;
  label: string;
  bundle: string;
  component: string;
}

const pages = ref<PluginPage[]>([]);
const loaded = ref(false);

export function usePlugins() {
  async function load(router: Router) {
    if (loaded.value) return;
    try {
      const res = await fetch("/api/plugins/pages", { credentials: "include" });
      if (!res.ok) return;
      const data = await res.json();
      pages.value = data.data || [];

      // Register each plugin page as a dynamic route
      for (const page of pages.value) {
        router.addRoute("desk", {
          path: page.route.replace("/app/", ""),
          name: `plugin-${page.route}`,
          component: () => import("@/components/PluginPage.vue"),
          props: { page },
        });
      }
    } catch {
      // No plugin pages
    } finally {
      loaded.value = true;
    }
  }

  return { pages, loaded, load };
}
