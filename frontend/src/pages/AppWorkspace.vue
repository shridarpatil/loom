<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import { loom } from "@/utils/call";
import { LPageHeader, LButton } from "@/components/ui";

const props = defineProps<{ appName: string }>();
const router = useRouter();

interface WorkspaceItem {
  label: string;
  route: string;
  icon?: string;
}

interface DashboardWidget {
  type: "number" | "shortcut" | "list";
  label: string;
  doctype?: string;
  filters?: Record<string, unknown>;
  route?: string;
  color?: string;
}

interface AppInfo {
  name: string;
  title: string;
  icon?: string;
  color?: string;
  workspace?: WorkspaceItem[];
  dashboard?: DashboardWidget[];
}

const app = ref<AppInfo | null>(null);
const loading = ref(true);
const editMode = ref(false);
const widgets = ref<DashboardWidget[]>([]);
const widgetData = ref<Record<string, number>>({});

// Add widget modal
const showAddWidget = ref(false);
const newWidget = ref<DashboardWidget>({ type: "number", label: "", doctype: "" });
const doctypes = ref<string[]>([]);

const iconPaths: Record<string, string> = {
  grid: "M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6ZM3.75 15.75A2.25 2.25 0 0 1 6 13.5h2.25a2.25 2.25 0 0 1 2.25 2.25V18a2.25 2.25 0 0 1-2.25 2.25H6A2.25 2.25 0 0 1 3.75 18v-2.25ZM13.5 6a2.25 2.25 0 0 1 2.25-2.25H18A2.25 2.25 0 0 1 20.25 6v2.25A2.25 2.25 0 0 1 18 10.5h-2.25a2.25 2.25 0 0 1-2.25-2.25V6ZM13.5 15.75a2.25 2.25 0 0 1 2.25-2.25H18a2.25 2.25 0 0 1 2.25 2.25V18A2.25 2.25 0 0 1 18 20.25h-2.25a2.25 2.25 0 0 1-2.25-2.25v-2.25Z",
  document: "M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z",
  users: "M15 19.128a9.38 9.38 0 0 0 2.625.372 9.337 9.337 0 0 0 4.121-.952 4.125 4.125 0 0 0-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 0 1 8.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0 1 11.964-3.07M12 6.375a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0Zm8.25 2.25a2.625 2.625 0 1 1-5.25 0 2.625 2.625 0 0 1 5.25 0Z",
  settings: "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z",
  shield: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z",
  lock: "M16.5 10.5V6.75a4.5 4.5 0 1 0-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H6.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z",
  chart: "M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z",
};

function getIconPath(name?: string): string {
  return iconPaths[name || "grid"] || iconPaths.grid;
}

async function loadApp() {
  loading.value = true;
  try {
    const res = await fetch("/api/apps", { credentials: "include" });
    if (res.ok) {
      const data = await res.json();
      const apps: AppInfo[] = data.data || [];
      app.value = apps.find((a) => a.name === props.appName) || null;
    }

    // Load user's saved dashboard widgets
    const wsRes = await fetch(`/api/settings/dashboard:${props.appName}`, { credentials: "include" });
    if (wsRes.ok) {
      const wsData = await wsRes.json();
      if (wsData.data?.widgets) {
        widgets.value = wsData.data.widgets;
      } else if (app.value?.dashboard) {
        widgets.value = app.value.dashboard;
      }
    } else if (app.value?.dashboard) {
      widgets.value = app.value.dashboard;
    }

    // Load DocType list for add widget
    const dtRes = await loom.resource("DocType").getList({ limit: 100 });
    doctypes.value = (dtRes.data as Array<{ name: string }>).map((d) => d.name).filter((n) => n !== "DocType");

    // Fetch counts for number widgets
    for (const w of widgets.value) {
      if (w.type === "number" && w.doctype) {
        fetchCount(w);
      }
    }
  } catch { /* */ }
  loading.value = false;
}

async function fetchCount(w: DashboardWidget) {
  if (!w.doctype) return;
  try {
    const params: Record<string, string> = { limit: "0" };
    if (w.filters) params.filters = JSON.stringify(w.filters);
    const res = await loom.resource(w.doctype).getList({ ...w.filters ? { filters: w.filters } : {}, limit: 10000 });
    widgetData.value[w.label] = res.data.length;
  } catch {
    widgetData.value[w.label] = 0;
  }
}

function addWidget() {
  if (!newWidget.value.label) return;
  widgets.value.push({ ...newWidget.value });
  newWidget.value = { type: "number", label: "", doctype: "" };
  showAddWidget.value = false;
  if (newWidget.value.type === "number" && newWidget.value.doctype) {
    fetchCount(widgets.value[widgets.value.length - 1]);
  }
}

function removeWidget(index: number) {
  widgets.value.splice(index, 1);
}

async function saveDashboard() {
  await fetch(`/api/settings/dashboard:${props.appName}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    credentials: "include",
    body: JSON.stringify({ widgets: widgets.value }),
  });
  editMode.value = false;
}

onMounted(loadApp);
watch(() => props.appName, loadApp);
</script>

<template>
  <div class="h-full flex flex-col">
    <LPageHeader :title="app?.title || appName">
      <template #breadcrumb>
        <button
          class="inline-flex items-center gap-0.5 text-[12px] text-text-muted hover:text-primary-600 transition-colors"
          @click="router.push('/app')"
        >
          <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5" />
          </svg>
          Home
        </button>
      </template>
      <template #actions>
        <template v-if="editMode">
          <LButton variant="secondary" size="sm" @click="showAddWidget = true">+ Add Widget</LButton>
          <LButton size="sm" @click="saveDashboard">Done</LButton>
        </template>
        <LButton v-else variant="secondary" size="sm" @click="editMode = true">Customize</LButton>
      </template>
    </LPageHeader>

    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <svg class="w-5 h-5 animate-spin text-text-light" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
    </div>

    <div v-else class="flex-1 overflow-auto px-6 py-6">
      <!-- Workspace icons -->
      <div v-if="app?.workspace && app.workspace.length > 0" class="flex flex-wrap justify-center gap-6 pb-8">
        <button
          v-for="item in app.workspace"
          :key="item.route"
          class="flex flex-col items-center gap-2.5 w-[88px] group"
          @click="router.push(item.route)"
        >
          <div
            class="w-[52px] h-[52px] rounded-2xl flex items-center justify-center shadow-sm transition-transform group-hover:scale-105 group-hover:shadow-md"
            :style="{ backgroundColor: (app.color || '#6366f1') + '18', color: app.color || '#6366f1' }"
          >
            <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" :d="getIconPath(item.icon)" />
            </svg>
          </div>
          <span class="text-[12px] text-text-muted font-medium text-center leading-tight group-hover:text-text transition-colors">{{ item.label }}</span>
        </button>
      </div>

      <!-- Dashboard widgets -->
      <div v-if="widgets.length > 0">
        <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
          <div
            v-for="(widget, wi) in widgets"
            :key="wi"
            :class="[
              'bg-white border rounded-xl px-5 py-4 relative group transition-all',
              editMode ? 'border-dashed border-border' : 'border-border/60 shadow-sm shadow-black/[0.02]',
            ]"
          >
            <!-- Remove button in edit mode -->
            <button
              v-if="editMode"
              class="absolute top-2 right-2 p-0.5 text-text-light hover:text-danger opacity-0 group-hover:opacity-100 transition-opacity"
              @click="removeWidget(wi)"
            >
              <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>

            <!-- Number widget -->
            <template v-if="widget.type === 'number'">
              <div
                class="text-2xl font-bold tracking-tight cursor-pointer hover:text-primary-600 transition-colors"
                :style="{ color: widget.color || undefined }"
                @click="widget.doctype ? router.push(`/app/${widget.doctype}`) : undefined"
              >{{ widgetData[widget.label] ?? '—' }}</div>
              <div class="text-[12px] text-text-muted mt-0.5">{{ widget.label }}</div>
            </template>

            <!-- Shortcut widget -->
            <template v-else-if="widget.type === 'shortcut'">
              <button
                class="flex items-center gap-2 text-[13px] font-medium text-primary-600 hover:text-primary-700"
                @click="widget.route ? router.push(widget.route) : undefined"
              >
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 6H5.25A2.25 2.25 0 0 0 3 8.25v10.5A2.25 2.25 0 0 0 5.25 21h10.5A2.25 2.25 0 0 0 18 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
                </svg>
                {{ widget.label }}
              </button>
            </template>
          </div>
        </div>
      </div>

      <!-- Empty state (no workspace and no widgets) -->
      <div v-if="(!app?.workspace || app.workspace.length === 0) && widgets.length === 0" class="flex-1 flex items-center justify-center py-12">
        <div class="text-center">
          <p class="text-[13px] text-text-light">No workspace configured</p>
          <LButton variant="secondary" size="sm" class="mt-2" @click="editMode = true">Add Widgets</LButton>
        </div>
      </div>
    </div>

    <!-- Add Widget Modal -->
    <Teleport to="body">
      <div v-if="showAddWidget" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/30 backdrop-blur-[2px]" @click="showAddWidget = false" />
        <div class="relative bg-white rounded-xl shadow-2xl border border-border/50 w-[400px] animate-scale-in">
          <div class="px-5 pt-5 pb-0 flex items-center justify-between">
            <h3 class="text-[15px] font-semibold">Add Widget</h3>
            <button class="p-1 rounded-md text-text-light hover:text-text hover:bg-surface-raised" @click="showAddWidget = false">
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
            </button>
          </div>
          <div class="px-5 py-4 space-y-3">
            <div>
              <label class="block text-[12px] font-medium text-text-muted mb-1">Type</label>
              <select v-model="newWidget.type" class="w-full h-9 px-3 text-[13px] border border-border rounded-lg">
                <option value="number">Number Card</option>
                <option value="shortcut">Shortcut</option>
              </select>
            </div>
            <div>
              <label class="block text-[12px] font-medium text-text-muted mb-1">Label</label>
              <input v-model="newWidget.label" type="text" placeholder="e.g. Open Todos" class="w-full h-9 px-3 text-[13px] border border-border rounded-lg" />
            </div>
            <div v-if="newWidget.type === 'number'">
              <label class="block text-[12px] font-medium text-text-muted mb-1">DocType</label>
              <select v-model="newWidget.doctype" class="w-full h-9 px-3 text-[13px] border border-border rounded-lg">
                <option value="">Select...</option>
                <option v-for="dt in doctypes" :key="dt" :value="dt">{{ dt }}</option>
              </select>
            </div>
            <div v-if="newWidget.type === 'shortcut'">
              <label class="block text-[12px] font-medium text-text-muted mb-1">Route</label>
              <input v-model="newWidget.route" type="text" placeholder="/app/Todo/new" class="w-full h-9 px-3 text-[13px] border border-border rounded-lg" />
            </div>
          </div>
          <div class="px-5 pb-5 flex justify-end gap-2">
            <LButton variant="secondary" @click="showAddWidget = false">Cancel</LButton>
            <LButton :disabled="!newWidget.label" @click="addWidget">Add</LButton>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
