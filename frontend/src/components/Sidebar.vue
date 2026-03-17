<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useSidebar } from "@/composables/useSidebar";
import { useTheme } from "@/composables/useTheme";
import { useSession } from "@/composables/useSession";
import { usePlugins } from "@/composables/usePlugins";

const router = useRouter();
const route = useRoute();
const { sections, pinnedItems, hiddenItems, loading, load, pin, unpin, hide } = useSidebar();
const { theme } = useTheme();
const { user, logout } = useSession();
const { pages: pluginPages } = usePlugins();
const collapsed = ref(false);
const contextMenu = ref<{ x: number; y: number; item: any } | null>(null);

onMounted(() => {
  load();
  document.addEventListener("click", () => { contextMenu.value = null; });
});

async function doLogout() {
  await logout();
  router.replace("/login");
}

function navigate(path: string) {
  router.push(path);
}

function isActive(path: string): boolean {
  return route.path === path || route.path.startsWith(path + "/");
}

function onContextMenu(e: MouseEvent, item: any) {
  e.preventDefault();
  contextMenu.value = { x: e.clientX, y: e.clientY, item };
}

// Group by module for collapsible sections
const groupedSections = computed(() => {
  return sections.value.map((s) => ({
    ...s,
    items: s.items.filter((item: any) => !hiddenItems.value.has(item.name)),
  })).filter((s) => s.items.length > 0);
});

const collapsedSections = ref<Set<string>>(new Set());
function toggleSection(label: string) {
  if (collapsedSections.value.has(label)) {
    collapsedSections.value.delete(label);
  } else {
    collapsedSections.value.add(label);
  }
  collapsedSections.value = new Set(collapsedSections.value);
}

const navItems = [
  { label: "Home", path: "/app", exact: true, icon: "M2.25 12l8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25" },
  { label: "Report Builder", path: "/app/report-builder", icon: "M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125Z" },
  { label: "Permissions", path: "/app/role-permission-manager", icon: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z" },
];
</script>

<template>
  <aside
    :class="[
      'h-screen bg-white border-r border-border flex flex-col transition-all duration-200 select-none shrink-0',
      collapsed ? 'w-[52px]' : 'w-[220px]',
    ]"
  >
    <!-- Header -->
    <div class="flex items-center h-12 px-3 shrink-0 border-b border-border/60">
      <button
        class="flex items-center justify-center w-7 h-7 rounded-lg hover:bg-surface-raised text-text-muted transition-all"
        @click="collapsed = !collapsed"
        title="Toggle sidebar"
      >
        <svg class="w-[18px] h-[18px]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
        </svg>
      </button>
      <template v-if="!collapsed">
        <img v-if="theme.logo_url" :src="theme.logo_url" :alt="theme.brand_name" class="ml-2 h-5" />
        <span v-else class="ml-2 font-bold text-[14px] tracking-tight text-primary-600">
          {{ theme.brand_name }}
        </span>
      </template>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 overflow-y-auto px-2 py-2 space-y-0.5">
      <!-- Main nav items -->
      <button
        v-for="item in navItems"
        :key="item.path"
        :class="[
          'w-full flex items-center gap-2.5 px-2.5 py-[7px] text-[13px] rounded-lg transition-all duration-150',
          (item.exact ? route.path === item.path : isActive(item.path))
            ? 'bg-primary-50 text-primary-700 font-medium shadow-sm shadow-primary-100'
            : 'text-text-muted hover:bg-surface-muted hover:text-text'
        ]"
        @click="navigate(item.path)"
      >
        <svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" :d="item.icon" />
        </svg>
        <span v-if="!collapsed">{{ item.label }}</span>
      </button>

      <!-- Plugin Pages -->
      <template v-if="pluginPages.length > 0">
        <div v-if="!collapsed" class="pt-3 pb-1 px-2.5">
          <span class="text-[10px] font-semibold text-text-light/70 uppercase tracking-widest">Apps</span>
        </div>
        <div v-else class="border-t border-border/40 mx-2 my-2" />
        <button
          v-for="pg in pluginPages"
          :key="pg.route"
          :class="[
            'w-full flex items-center gap-2.5 px-2.5 py-[7px] text-[13px] rounded-lg transition-all duration-150',
            isActive(pg.route) ? 'bg-primary-50 text-primary-700 font-medium' : 'text-text-muted hover:bg-surface-muted hover:text-text'
          ]"
          @click="navigate(pg.route)"
          :title="pg.label"
        >
          <span class="w-4 h-4 shrink-0 rounded-md bg-surface-raised flex items-center justify-center text-[9px] font-bold text-text-light">
            {{ pg.label.charAt(0) }}
          </span>
          <span v-if="!collapsed" class="truncate">{{ pg.label }}</span>
        </button>
      </template>

      <!-- Pinned items -->
      <template v-if="pinnedItems.length > 0">
        <div v-if="!collapsed" class="pt-3 pb-1 px-2.5">
          <span class="text-[10px] font-semibold text-text-light/70 uppercase tracking-widest">Pinned</span>
        </div>
        <div v-else class="border-t border-border/40 mx-2 my-2" />
        <button
          v-for="item in pinnedItems"
          :key="'pin-' + item.name"
          :class="[
            'w-full flex items-center gap-2.5 px-2.5 py-[7px] text-[13px] rounded-lg transition-all duration-150',
            isActive(item.route) ? 'bg-primary-50 text-primary-700 font-medium' : 'text-text-muted hover:bg-surface-muted hover:text-text'
          ]"
          @click="navigate(item.route)"
          @contextmenu="onContextMenu($event, { ...item, pinned: true })"
          :title="item.name"
        >
          <span class="w-4 h-4 shrink-0 rounded-md bg-primary-50 flex items-center justify-center text-[9px] font-bold text-primary-600">
            {{ item.name.charAt(0) }}
          </span>
          <span v-if="!collapsed" class="truncate">{{ item.name }}</span>
        </button>
      </template>

      <!-- Module sections -->
      <template v-if="groupedSections.length > 0">
        <template v-for="section in groupedSections" :key="section.label">
          <div v-if="!collapsed" class="pt-3 pb-1 px-2.5">
            <button
              class="w-full flex items-center gap-1.5 text-[10px] font-semibold text-text-light/70 uppercase tracking-widest hover:text-text-muted transition-colors"
              @click="toggleSection(section.label)"
            >
              <svg
                :class="['w-2.5 h-2.5 transition-transform duration-200', collapsedSections.has(section.label) ? '' : 'rotate-90']"
                fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5"
              >
                <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
              </svg>
              {{ section.label }}
            </button>
          </div>
          <div v-else class="border-t border-border/40 mx-2 my-2" />

          <template v-if="!collapsedSections.has(section.label)">
            <button
              v-for="item in section.items"
              :key="item.name"
              :class="[
                'w-full flex items-center gap-2.5 px-2.5 py-[7px] text-[13px] rounded-lg transition-all duration-150',
                isActive(item.route) ? 'bg-primary-50 text-primary-700 font-medium' : 'text-text-muted hover:bg-surface-muted hover:text-text'
              ]"
              @click="navigate(item.route)"
              @contextmenu="onContextMenu($event, item)"
              :title="item.name"
            >
              <span class="w-4 h-4 shrink-0 rounded-md bg-surface-raised flex items-center justify-center text-[9px] font-bold text-text-light">
                {{ item.name.charAt(0) }}
              </span>
              <span v-if="!collapsed" class="truncate">{{ item.name }}</span>
            </button>
          </template>
        </template>
      </template>

      <!-- Loading skeleton -->
      <template v-if="loading && groupedSections.length === 0">
        <div v-for="i in 4" :key="i" class="flex items-center gap-2.5 px-2.5 py-2">
          <div class="w-4 h-4 rounded-md bg-surface-raised animate-pulse shrink-0" />
          <div v-if="!collapsed" class="h-3 rounded bg-surface-raised animate-pulse" :style="{ width: `${40 + i * 20}px` }" />
        </div>
      </template>

      <div
        v-if="!loading && groupedSections.length === 0 && pinnedItems.length === 0 && !collapsed"
        class="px-2.5 py-6 text-center"
      >
        <p class="text-[12px] text-text-light">No DocTypes yet</p>
        <button class="mt-1 text-[12px] text-primary-600 hover:text-primary-700" @click="navigate('/app/DocType/new')">Create one</button>
      </div>
    </nav>

    <!-- Footer -->
    <div class="px-2 py-2 border-t border-border/60 shrink-0">
      <div class="flex items-center gap-2 px-1.5 py-1">
        <div class="w-6 h-6 rounded-full bg-gradient-to-br from-primary-400 to-primary-600 flex items-center justify-center shrink-0 shadow-sm">
          <span class="text-[10px] font-bold text-white">{{ user.charAt(0).toUpperCase() }}</span>
        </div>
        <span v-if="!collapsed" class="text-[12px] text-text-muted truncate flex-1">{{ user }}</span>
        <button
          v-if="!collapsed"
          class="p-1 rounded-md text-text-light hover:text-text hover:bg-surface-raised transition-all shrink-0"
          title="Sign out"
          @click="doLogout"
        >
          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div
        v-if="contextMenu"
        class="fixed z-50 bg-white border border-border rounded-xl shadow-xl shadow-black/8 py-1 min-w-[150px] animate-scale-in"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      >
        <button
          v-if="!contextMenu.item.pinned"
          class="w-full text-left px-3 py-2 text-[13px] text-text-muted hover:bg-surface-muted hover:text-text transition-colors"
          @click="pin(contextMenu.item); contextMenu = null"
        >Pin to top</button>
        <button
          v-if="contextMenu.item.pinned"
          class="w-full text-left px-3 py-2 text-[13px] text-text-muted hover:bg-surface-muted hover:text-text transition-colors"
          @click="unpin(contextMenu.item.name); contextMenu = null"
        >Unpin</button>
        <button
          class="w-full text-left px-3 py-2 text-[13px] text-text-muted hover:bg-surface-muted hover:text-text transition-colors"
          @click="hide(contextMenu.item.name); contextMenu = null"
        >Hide</button>
      </div>
    </Teleport>
  </aside>
</template>
