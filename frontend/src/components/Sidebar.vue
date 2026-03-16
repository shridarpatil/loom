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

// Context menu
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
    items: s.items.filter((item) => !hiddenItems.value.has(item.name)),
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
</script>

<template>
  <aside
    :class="[
      'h-screen bg-white border-r border-border flex flex-col transition-all duration-150 select-none',
      collapsed ? 'w-[48px]' : 'w-[200px]',
    ]"
  >
    <!-- Header -->
    <div class="flex items-center h-11 px-2.5 shrink-0 border-b border-border">
      <button
        class="flex items-center gap-1.5 p-1 rounded hover:bg-surface-raised text-text-muted transition-colors"
        @click="collapsed = !collapsed"
        title="Toggle sidebar"
      >
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
        </svg>
      </button>
      <template v-if="!collapsed">
        <img v-if="theme.logo_url" :src="theme.logo_url" :alt="theme.brand_name" class="ml-1.5 h-4" />
        <span v-else class="ml-1.5 font-semibold text-[13px] tracking-tight text-primary-600">
          {{ theme.brand_name }}
        </span>
      </template>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 overflow-y-auto px-1.5 py-1 space-y-px">
      <!-- Home -->
      <button
        :class="[
          'w-full flex items-center gap-2 px-2 py-1.5 text-[13px] rounded transition-colors',
          isActive('/app') && route.path === '/app'
            ? 'bg-primary-50 text-primary-700 font-medium'
            : 'text-text-muted hover:bg-surface-raised hover:text-text'
        ]"
        @click="navigate('/app')"
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="m2.25 12 8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25" />
        </svg>
        <span v-if="!collapsed">Home</span>
      </button>

      <!-- Report Builder -->
      <button
        :class="[
          'w-full flex items-center gap-2 px-2 py-1.5 text-[13px] rounded transition-colors',
          isActive('/app/report-builder')
            ? 'bg-primary-50 text-primary-700 font-medium'
            : 'text-text-muted hover:bg-surface-raised hover:text-text'
        ]"
        @click="navigate('/app/report-builder')"
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z" />
        </svg>
        <span v-if="!collapsed">Report Builder</span>
      </button>

      <!-- Role Permission Manager -->
      <button
        :class="[
          'w-full flex items-center gap-2 px-2 py-1.5 text-[13px] rounded transition-colors',
          isActive('/app/role-permission-manager')
            ? 'bg-primary-50 text-primary-700 font-medium'
            : 'text-text-muted hover:bg-surface-raised hover:text-text'
        ]"
        @click="navigate('/app/role-permission-manager')"
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z" />
        </svg>
        <span v-if="!collapsed">Permissions</span>
      </button>

      <!-- Plugin Pages -->
      <template v-if="pluginPages.length > 0">
        <div v-if="!collapsed" class="pt-2.5 pb-1 px-2">
          <span class="text-[10px] font-semibold text-text-light uppercase tracking-wider">Apps</span>
        </div>
        <div v-else class="border-t border-border mx-1 my-1.5" />
        <button
          v-for="pg in pluginPages"
          :key="pg.route"
          :class="[
            'w-full flex items-center gap-2 px-2 py-1.5 text-[13px] rounded transition-colors',
            isActive(pg.route)
              ? 'bg-primary-50 text-primary-700 font-medium'
              : 'text-text-muted hover:bg-surface-raised hover:text-text'
          ]"
          @click="navigate(pg.route)"
          :title="pg.label"
        >
          <span class="w-3.5 h-3.5 shrink-0 rounded bg-surface-raised flex items-center justify-center text-[9px] font-bold text-text-light">
            {{ pg.label.charAt(0) }}
          </span>
          <span v-if="!collapsed" class="truncate">{{ pg.label }}</span>
        </button>
      </template>

      <!-- Pinned items -->
      <template v-if="pinnedItems.length > 0">
        <div v-if="!collapsed" class="pt-2.5 pb-1 px-2">
          <span class="text-[10px] font-semibold text-text-light uppercase tracking-wider">Pinned</span>
        </div>
        <div v-else class="border-t border-border mx-1 my-1.5" />
        <button
          v-for="item in pinnedItems"
          :key="'pin-' + item.name"
          :class="[
            'w-full flex items-center gap-2 px-2 py-1.5 text-[13px] rounded transition-colors',
            isActive(item.route)
              ? 'bg-primary-50 text-primary-700 font-medium'
              : 'text-text-muted hover:bg-surface-raised hover:text-text'
          ]"
          @click="navigate(item.route)"
          @contextmenu="onContextMenu($event, { ...item, pinned: true })"
          :title="item.name"
        >
          <span class="w-3.5 h-3.5 shrink-0 rounded bg-primary-100 flex items-center justify-center text-[9px] font-bold text-primary-600">
            {{ item.name.charAt(0) }}
          </span>
          <span v-if="!collapsed" class="truncate">{{ item.name }}</span>
        </button>
      </template>

      <!-- Module sections -->
      <template v-if="groupedSections.length > 0">
        <template v-for="section in groupedSections" :key="section.label">
          <div v-if="!collapsed" class="pt-2.5 pb-1 px-2">
            <button
              class="w-full flex items-center gap-1 text-[10px] font-semibold text-text-light uppercase tracking-wider hover:text-text-muted transition-colors"
              @click="toggleSection(section.label)"
            >
              <svg
                :class="['w-2.5 h-2.5 transition-transform', collapsedSections.has(section.label) ? '' : 'rotate-90']"
                fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
              >
                <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
              </svg>
              {{ section.label }}
            </button>
          </div>
          <div v-else class="border-t border-border mx-1 my-1.5" />

          <template v-if="!collapsedSections.has(section.label)">
            <button
              v-for="item in section.items"
              :key="item.name"
              :class="[
                'w-full flex items-center gap-2 px-2 py-1.5 text-[13px] rounded transition-colors',
                isActive(item.route)
                  ? 'bg-primary-50 text-primary-700 font-medium'
                  : 'text-text-muted hover:bg-surface-raised hover:text-text'
              ]"
              @click="navigate(item.route)"
              @contextmenu="onContextMenu($event, item)"
              :title="item.name"
            >
              <span class="w-3.5 h-3.5 shrink-0 rounded bg-surface-raised flex items-center justify-center text-[9px] font-bold text-text-light">
                {{ item.name.charAt(0) }}
              </span>
              <span v-if="!collapsed" class="truncate">{{ item.name }}</span>
            </button>
          </template>
        </template>
      </template>

      <!-- Loading skeleton -->
      <template v-if="loading && groupedSections.length === 0">
        <div v-for="i in 3" :key="i" class="flex items-center gap-2 px-2 py-1.5">
          <div class="w-3.5 h-3.5 rounded bg-surface-raised animate-pulse shrink-0" />
          <div v-if="!collapsed" class="h-2.5 rounded bg-surface-raised animate-pulse" :style="{ width: `${50 + i * 18}px` }" />
        </div>
      </template>

      <div
        v-if="!loading && groupedSections.length === 0 && pinnedItems.length === 0 && !collapsed"
        class="px-2 py-1.5 text-[11px] text-text-light"
      >
        No DocTypes yet
      </div>
    </nav>

    <!-- Footer -->
    <div class="px-2 py-1.5 border-t border-border shrink-0">
      <div class="flex items-center gap-1.5 px-1">
        <div class="w-5 h-5 rounded-full bg-primary-100 flex items-center justify-center shrink-0">
          <span class="text-[9px] font-bold text-primary-700">{{ user.charAt(0).toUpperCase() }}</span>
        </div>
        <span v-if="!collapsed" class="text-[11px] text-text-muted truncate flex-1">{{ user }}</span>
        <button
          v-if="!collapsed"
          class="p-0.5 text-text-light hover:text-text transition-colors shrink-0"
          title="Sign out"
          @click="doLogout"
        >
          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div
        v-if="contextMenu"
        class="fixed z-50 bg-white border border-border rounded-lg shadow-lg py-1 min-w-[140px]"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      >
        <button
          v-if="!contextMenu.item.pinned"
          class="w-full text-left px-3 py-1.5 text-[12px] text-text-muted hover:bg-surface-raised transition-colors"
          @click="pin(contextMenu.item); contextMenu = null"
        >Pin to top</button>
        <button
          v-if="contextMenu.item.pinned"
          class="w-full text-left px-3 py-1.5 text-[12px] text-text-muted hover:bg-surface-raised transition-colors"
          @click="unpin(contextMenu.item.name); contextMenu = null"
        >Unpin</button>
        <button
          class="w-full text-left px-3 py-1.5 text-[12px] text-text-muted hover:bg-surface-raised transition-colors"
          @click="hide(contextMenu.item.name); contextMenu = null"
        >Hide</button>
      </div>
    </Teleport>
  </aside>
</template>
