<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useTheme } from "@/composables/useTheme";
import { useSession } from "@/composables/useSession";

const router = useRouter();
const route = useRoute();
const { theme } = useTheme();
const { user, logout } = useSession();
const collapsed = ref(false);

interface WorkspaceItem {
  label: string;
  route: string;
  icon?: string;
  display?: "icons" | "sidebar" | "both";
}

interface AppInfo {
  name: string;
  title: string;
  icon?: string;
  color?: string;
  modules?: string[];
  workspace?: WorkspaceItem[];
}

const installedApps = ref<AppInfo[]>([]);
const allDoctypes = ref<Array<{ name: string; module: string }>>([]);

// Icon paths
const iconPaths: Record<string, string> = {
  home: "M2.25 12l8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25",
  grid: "M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6ZM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25ZM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6ZM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25a2.25 2.25 0 01-2.25-2.25v-2.25Z",
  document: "M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9Z",
  users: "M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0Zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0Z",
  settings: "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 010 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 010-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28ZM15 12a3 3 0 11-6 0 3 3 0 016 0Z",
  shield: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z",
  lock: "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25Z",
  chart: "M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125Z",
};

function getIconPath(name?: string): string {
  return iconPaths[name || "grid"] || iconPaths.grid;
}

// Detect which app the user is currently in based on the route
const currentApp = computed<AppInfo | null>(() => {
  const path = route.path;

  // Check if path matches an app name directly: /app/{app_name}
  for (const app of installedApps.value) {
    if (path === `/app/${app.name}` || path.startsWith(`/app/${app.name}/`)) {
      return app;
    }
  }

  // Check if path matches a DocType that belongs to an app's module
  const pathParts = path.split("/");
  if (pathParts.length >= 3 && pathParts[1] === "app") {
    const dtName = decodeURIComponent(pathParts[2]);
    const dt = allDoctypes.value.find((d) => d.name === dtName);
    if (dt) {
      for (const app of installedApps.value) {
        if (app.modules?.includes(dt.module)) {
          return app;
        }
      }
    }
  }

  // Check if path matches any workspace item's route in an app
  // e.g., /app/report-builder or /app/role-permission-manager
  for (const app of installedApps.value) {
    if (app.workspace) {
      for (const item of app.workspace) {
        if (path === item.route || path.startsWith(item.route + "/")) {
          return app;
        }
      }
    }
  }

  return null;
});

const isHome = computed(() => route.path === "/app");

// Sidebar items: filtered by display mode (show if "sidebar" or "both" or unset)
const sidebarItems = computed<WorkspaceItem[]>(() => {
  if (currentApp.value?.workspace) {
    return currentApp.value.workspace.filter(
      (item) => !item.display || item.display === "both" || item.display === "sidebar"
    );
  }
  return [];
});

function isActive(path: string): boolean {
  return route.path === path || route.path.startsWith(path + "/");
}

function navigate(path: string) {
  router.push(path);
}

async function doLogout() {
  await logout();
  router.replace("/login");
}

onMounted(async () => {
  try {
    const [appsRes, dtRes] = await Promise.all([
      fetch("/api/apps", { credentials: "include" }),
      fetch("/api/resource/DocType?fields=[\"id\",\"module\"]&limit=200", { credentials: "include" }),
    ]);

    if (appsRes.ok) {
      const data = await appsRes.json();
      installedApps.value = data.data || [];
    }

    if (dtRes.ok) {
      const data = await dtRes.json();
      allDoctypes.value = (data.data || []).map((d: any) => ({
        name: d.id || d.name,
        module: d.module || "",
      }));
    }
  } catch { /* */ }
});
</script>

<template>
  <aside
    :class="[
      'h-full bg-white border-r border-border flex flex-col transition-all duration-200 select-none shrink-0',
      collapsed ? 'w-[52px]' : 'w-[220px]',
    ]"
  >
    <!-- Navigation -->
    <nav class="flex-1 overflow-y-auto px-2 py-3 space-y-0.5">
      <!-- App workspace link — goes to current app's workspace, not global home -->
      <button
        v-if="currentApp"
        :class="[
          'w-full flex items-center gap-2.5 px-2.5 py-[7px] text-[13px] rounded-lg transition-all duration-150',
          route.path === `/app/${currentApp.name}`
            ? 'bg-primary-50 text-primary-700 font-medium shadow-sm shadow-primary-100'
            : 'text-text-muted hover:bg-surface-muted hover:text-text'
        ]"
        @click="navigate(`/app/${currentApp.name}`)"
      >
        <svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" :d="iconPaths.home" />
        </svg>
        <span v-if="!collapsed">{{ currentApp.title }}</span>
      </button>

      <!-- Divider when inside an app -->
      <div v-if="sidebarItems.length > 0" class="border-t border-border/40 mx-2 my-2" />

      <!-- App workspace items -->
      <button
        v-for="item in sidebarItems"
        :key="item.route"
        :class="[
          'w-full flex items-center gap-2.5 px-2.5 py-[7px] text-[13px] rounded-lg transition-all duration-150',
          isActive(item.route)
            ? 'bg-primary-50 text-primary-700 font-medium shadow-sm shadow-primary-100'
            : 'text-text-muted hover:bg-surface-muted hover:text-text'
        ]"
        @click="navigate(item.route)"
      >
        <svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" :d="getIconPath(item.icon)" />
        </svg>
        <span v-if="!collapsed">{{ item.label }}</span>
      </button>

      <!-- Empty state on home -->
      <div
        v-if="isHome && sidebarItems.length === 0 && !collapsed"
        class="px-2.5 py-6 text-center"
      >
        <p class="text-[12px] text-text-light">Select an app</p>
      </div>
    </nav>

    <!-- Collapse toggle at bottom -->
    <div class="px-2 py-2 shrink-0">
      <button
        class="w-full flex items-center justify-center gap-2 px-2.5 py-[6px] text-[12px] text-text-light hover:text-text-muted hover:bg-surface-muted rounded-lg transition-all"
        @click="collapsed = !collapsed"
        :title="collapsed ? 'Expand sidebar' : 'Collapse sidebar'"
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path v-if="collapsed" stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
          <path v-else stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5" />
        </svg>
        <span v-if="!collapsed">Collapse</span>
      </button>
    </div>
  </aside>
</template>
