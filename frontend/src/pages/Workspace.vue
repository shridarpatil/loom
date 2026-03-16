<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { loom } from "@/utils/call";
import { useSession } from "@/composables/useSession";
import { LPageHeader, LButton } from "@/components/ui";
import WorkspaceGrid from "@/components/workspace/WorkspaceGrid.vue";

const router = useRouter();
const { user } = useSession();
const doctypes = ref<string[]>([]);
const recentCount = ref(0);
const editMode = ref(false);

// Widgets from workspace doc or defaults
const widgets = ref<Array<{ type: string; [key: string]: unknown }>>([]);

onMounted(async () => {
  try {
    const res = await loom.resource("DocType").getList({ limit: 100 });
    doctypes.value = (res.data as Array<{ name: string }>)
      .map((d) => d.name)
      .filter((n) => n !== "DocType");
    recentCount.value = doctypes.value.length;

    // Try loading workspace widgets from user settings
    try {
      const wsRes = await fetch("/api/settings/workspace:Home", { credentials: "include" });
      if (wsRes.ok) {
        const wsData = await wsRes.json();
        if (wsData.data?.widgets) {
          widgets.value = wsData.data.widgets;
          return;
        }
      }
    } catch {
      // No saved workspace, use defaults
    }

    // Default widgets
    widgets.value = [
      { type: "shortcut", label: "New DocType", route: "/app/DocType/new" },
      ...doctypes.value.slice(0, 2).map((dt) => ({
        type: "shortcut" as const,
        label: `New ${dt}`,
        route: `/app/${dt}/new`,
      })),
      ...doctypes.value.slice(0, 2).map((dt) => ({
        type: "count" as const,
        label: dt,
        doctype: dt,
      })),
    ];
  } catch {
    doctypes.value = [];
  }
});

function addWidget(type: string) {
  if (type === "shortcut") {
    widgets.value.push({ type: "shortcut", label: "New Shortcut", route: "/app" });
  } else if (type === "count") {
    const dt = doctypes.value[0] || "DocType";
    widgets.value.push({ type: "count", label: dt, doctype: dt });
  } else if (type === "list") {
    const dt = doctypes.value[0] || "DocType";
    widgets.value.push({ type: "list", label: `Recent ${dt}`, doctype: dt, limit: 5 });
  }
}

function removeWidget(index: number) {
  widgets.value.splice(index, 1);
}

async function toggleEditMode() {
  if (editMode.value) {
    // Leaving edit mode — save widgets
    try {
      await fetch("/api/settings/workspace:Home", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ widgets: widgets.value }),
      });
    } catch {
      // silently fail
    }
  }
  editMode.value = !editMode.value;
}
</script>

<template>
  <div class="h-full">
    <!-- Top bar -->
    <LPageHeader title="Home" :subtitle="`Welcome to Loom Desk`">
      <template #actions>
        <LButton
          :variant="editMode ? 'primary' : 'secondary'"
          size="sm"
          @click="toggleEditMode"
        >
          {{ editMode ? "Done" : "Edit" }}
        </LButton>
      </template>
    </LPageHeader>

    <div class="px-6 py-5">
      <!-- Stats row -->
      <div class="grid grid-cols-3 gap-3 mb-6">
        <div class="bg-white border border-border rounded-lg px-4 py-3">
          <div class="text-xl font-semibold">{{ recentCount }}</div>
          <div class="text-[12px] text-text-muted">DocTypes</div>
        </div>
        <div class="bg-white border border-border rounded-lg px-4 py-3">
          <div class="text-xl font-semibold text-emerald-600">Active</div>
          <div class="text-[12px] text-text-muted">Server Status</div>
        </div>
        <div class="bg-white border border-border rounded-lg px-4 py-3">
          <div class="text-xl font-semibold">{{ user }}</div>
          <div class="text-[12px] text-text-muted">Logged in as</div>
        </div>
      </div>

      <!-- Edit mode: add widget buttons -->
      <div v-if="editMode" class="mb-4 flex items-center gap-2">
        <span class="text-[11px] font-semibold text-text-light uppercase tracking-wider">Add Widget:</span>
        <LButton variant="secondary" size="sm" @click="addWidget('shortcut')">Shortcut</LButton>
        <LButton variant="secondary" size="sm" @click="addWidget('count')">Count</LButton>
        <LButton variant="secondary" size="sm" @click="addWidget('list')">List</LButton>
      </div>

      <!-- Dynamic widgets -->
      <div v-if="widgets.length > 0" class="mb-6">
        <h2 class="text-[11px] font-semibold text-text-light uppercase tracking-wider mb-2">Widgets</h2>
        <div v-if="editMode" class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          <div
            v-for="(widget, i) in widgets"
            :key="i"
            class="bg-white border border-dashed border-border rounded-lg px-4 py-3 relative group"
          >
            <div class="text-[12px] text-text-muted">{{ widget.type }}: {{ (widget as any).label || (widget as any).doctype }}</div>
            <button
              class="absolute top-1 right-1 p-0.5 text-text-light hover:text-danger opacity-0 group-hover:opacity-100 transition-opacity"
              @click="removeWidget(i)"
            >
              <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>
        <WorkspaceGrid v-else :widgets="widgets as any" />
      </div>

      <!-- DocType list -->
      <div v-if="doctypes.length > 0">
        <h2 class="text-[11px] font-semibold text-text-light uppercase tracking-wider mb-2">Your DocTypes</h2>
        <div class="bg-white border border-border rounded-lg overflow-hidden">
          <button
            v-for="dt in doctypes"
            :key="dt"
            class="w-full flex items-center gap-2.5 px-4 py-2.5 text-left border-b border-border last:border-0 hover:bg-surface-muted/50 transition-colors"
            @click="router.push(`/app/${dt}`)"
          >
            <span class="w-6 h-6 rounded bg-primary-50 flex items-center justify-center shrink-0 text-[10px] font-bold text-primary-600">
              {{ dt.charAt(0) }}
            </span>
            <span class="flex-1 text-[13px] font-medium">{{ dt }}</span>
            <svg class="w-3.5 h-3.5 text-text-light" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
