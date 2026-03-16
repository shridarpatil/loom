<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { loom, type DocTypeMeta, type DocFieldMeta } from "@/utils/call";
import { socket } from "@/utils/socket";
import { useSession } from "@/composables/useSession";
import { useListView, type SavedView } from "@/composables/useListView";
import { LButton, LBadge, LAlert, LPageHeader, LInput, LModal } from "@/components/ui";

const props = defineProps<{ doctype: string }>();
const router = useRouter();
const { isAdmin } = useSession();

const meta = ref<DocTypeMeta | null>(null);
const rows = ref<Record<string, unknown>[]>([]);
const loading = ref(true);
const error = ref("");
const page = ref(0);
const pageSize = 20;

// --- Sorting ---
const sortField = ref("modified");
const sortOrder = ref<"asc" | "desc">("desc");

function toggleSort(fieldname: string) {
  if (sortField.value === fieldname) {
    sortOrder.value = sortOrder.value === "asc" ? "desc" : "asc";
  } else {
    sortField.value = fieldname;
    sortOrder.value = "desc";
  }
  page.value = 0;
  loadData();
}

// --- Bulk selection ---
const selectedIds = ref<Set<string>>(new Set());
const selectAll = ref(false);

function toggleSelectAll() {
  if (selectAll.value) {
    selectedIds.value = new Set();
    selectAll.value = false;
  } else {
    selectedIds.value = new Set(rows.value.map((r) => String(r.id)));
    selectAll.value = true;
  }
}

function toggleSelect(id: string) {
  const s = new Set(selectedIds.value);
  if (s.has(id)) {
    s.delete(id);
  } else {
    s.add(id);
  }
  selectedIds.value = s;
  selectAll.value = s.size === rows.value.length && rows.value.length > 0;
}

function clearSelection() {
  selectedIds.value = new Set();
  selectAll.value = false;
}

async function bulkDelete() {
  if (selectedIds.value.size === 0) return;
  const count = selectedIds.value.size;
  if (!confirm(`Delete ${count} record${count > 1 ? "s" : ""}? This cannot be undone.`)) return;
  for (const id of selectedIds.value) {
    try {
      await loom.resource(props.doctype).delete(id);
    } catch {
      // continue deleting others
    }
  }
  clearSelection();
  loadData();
}

function exportCsv() {
  if (selectedIds.value.size === 0) return;
  const selected = rows.value.filter((r) => selectedIds.value.has(String(r.id)));
  const cols = ["id", ...listColumns.value.map((c) => c.fieldname)];
  const header = cols.join(",");
  const lines = selected.map((row) =>
    cols.map((c) => {
      const val = row[c];
      if (val === null || val === undefined) return "";
      const str = String(val);
      return str.includes(",") || str.includes('"') || str.includes("\n")
        ? `"${str.replace(/"/g, '""')}"`
        : str;
    }).join(","),
  );
  const csv = [header, ...lines].join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${props.doctype}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}

// Saved views
const { views, activeView, load: loadViews, addView, removeView, setActiveView, saveViews } = useListView(() => props.doctype);
const showSaveView = ref(false);
const newViewName = ref("");
const newViewDefault = ref(false);

// --- Filters ---
// Each active filter: { field, operator, value }
const activeFilters = ref<Array<{ field: string; operator: string; value: string }>>([]);

// All filterable data fields (excludes layout fields)
const allDataFields = computed<DocFieldMeta[]>(() => {
  if (!meta.value) return [];
  return meta.value.fields.filter(
    (f) => !["SectionBreak", "ColumnBreak", "TabBreak", "Table"].includes(f.fieldtype),
  );
});

// Fields shown as standard filters (auto-displayed)
const standardFilterFields = computed<DocFieldMeta[]>(() => {
  return allDataFields.value.filter((f) => f.in_standard_filter);
});

// Extra user-added filter fields (not in standard filters)
const extraFilterFieldnames = ref<string[]>([]);

// Combined visible filter fields = standard + extra
const visibleFilterFields = computed<DocFieldMeta[]>(() => {
  const standard = standardFilterFields.value;
  const standardNames = new Set(standard.map((f) => f.fieldname));
  const extra = extraFilterFieldnames.value
    .filter((fn) => !standardNames.has(fn))
    .map((fn) => allDataFields.value.find((f) => f.fieldname === fn))
    .filter(Boolean) as DocFieldMeta[];
  return [...standard, ...extra];
});

// Fields available to add (not already visible)
const addableFields = computed<DocFieldMeta[]>(() => {
  const visible = new Set(visibleFilterFields.value.map((f) => f.fieldname));
  return allDataFields.value.filter((f) => !visible.has(f.fieldname));
});

// Filter values keyed by fieldname
const filterValues = ref<Record<string, string>>({});

// Show/hide the "add filter" dropdown
const showAddFilter = ref(false);

function addFilterField(fieldname: string) {
  if (!extraFilterFieldnames.value.includes(fieldname)) {
    extraFilterFieldnames.value.push(fieldname);
  }
  showAddFilter.value = false;
}

function removeFilterField(fieldname: string) {
  extraFilterFieldnames.value = extraFilterFieldnames.value.filter((fn) => fn !== fieldname);
  delete filterValues.value[fieldname];
  rebuildAndLoad();
}

function onFilterChange(fieldname: string, value: string) {
  filterValues.value = { ...filterValues.value, [fieldname]: value };
  rebuildAndLoad();
}

function rebuildAndLoad() {
  const newFilters: Array<{ field: string; operator: string; value: string }> = [];
  for (const [fn, val] of Object.entries(filterValues.value)) {
    if (!val) continue;
    const field = allDataFields.value.find((f) => f.fieldname === fn);
    const ft = field?.fieldtype || "Data";
    const exactTypes = ["Select", "Check", "Link", "Date", "Datetime", "Int", "Float", "Currency"];
    const op = exactTypes.includes(ft) ? "=" : "like";
    const filterVal = op === "like" ? `%${val}%` : val;
    newFilters.push({ field: fn, operator: op, value: filterVal });
  }
  activeFilters.value = newFilters;
  page.value = 0;
  loadData();
}

function clearAllFilters() {
  filterValues.value = {};
  extraFilterFieldnames.value = [];
  activeFilters.value = [];
  page.value = 0;
  loadData();
}

function removeFilter(index: number) {
  const removed = activeFilters.value[index];
  activeFilters.value.splice(index, 1);
  if (removed) {
    delete filterValues.value[removed.field];
  }
  page.value = 0;
  loadData();
}

const hasActiveFilters = computed(() => Object.values(filterValues.value).some((v) => v));

// --- List columns ---
const listColumns = computed<DocFieldMeta[]>(() => {
  if (!meta.value) return [];
  const cols = meta.value.fields.filter((f) => f.in_list_view && f.fieldname !== "id");
  if (cols.length === 0) {
    return meta.value.fields
      .filter((f) => !["SectionBreak", "ColumnBreak", "TabBreak"].includes(f.fieldtype))
      .slice(0, 4);
  }
  return cols;
});

const fieldNames = computed(() => {
  const names = ["id", ...listColumns.value.map((f) => f.fieldname)];
  return [...new Set(names)];
});

// --- Custom buttons from client script ---
interface CustomButton {
  label: string;
  action: (selectedRows: Record<string, unknown>[]) => void;
  variant?: string;
  view?: string;
}
const customButtons = ref<CustomButton[]>([]);

function loadClientScript(script: string) {
  if (!script) { customButtons.value = []; return; }
  try {
    const buttons: CustomButton[] = [];
    const sandbox: Record<string, unknown> = {
      add_button(label: string, action: (selectedRows: Record<string, unknown>[]) => void, options?: { variant?: string; view?: string }) {
        buttons.push({ label, action, variant: options?.variant || "secondary", view: options?.view || "both" });
      },
    };
    const fn = new Function("loom", script);
    fn(sandbox);
    // Only show buttons meant for list view
    customButtons.value = buttons.filter((b) => b.view === "list" || b.view === "both");
  } catch (e) {
    console.error("Client script error:", e);
    customButtons.value = [];
  }
}

function runCustomButton(btn: CustomButton) {
  try {
    const selected = rows.value.filter((r) => selectedIds.value.has(String(r.id)));
    btn.action(selected);
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Button action failed";
  }
}

// --- Data loading ---
async function loadMeta() {
  try {
    const res = await loom.getMeta(props.doctype);
    meta.value = res.data;
    loadClientScript((res.data as any).client_script || "");
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Failed to load meta";
  }
}

async function loadData() {
  loading.value = true;
  error.value = "";
  try {
    const filterArr: unknown[] = activeFilters.value
      .filter((f) => f.value)
      .map((f) => [f.field, f.operator, f.value]);

    const res = await loom.resource(props.doctype).getList({
      fields: fieldNames.value,
      order_by: `${sortField.value} ${sortOrder.value}`,
      limit: pageSize,
      offset: page.value * pageSize,
      filters: filterArr.length > 0 ? filterArr : undefined,
    });
    rows.value = res.data;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Failed to load data";
  } finally {
    loading.value = false;
  }
}

// --- Saved views ---
function applyView(view: SavedView) {
  setActiveView(view.name);
  activeFilters.value = (view.filters || []).map((f: any) => ({
    field: f[0] || "",
    operator: f[1] || "=",
    value: f[2] || "",
  }));
  // Populate filterValues from view filters
  filterValues.value = {};
  for (const f of activeFilters.value) {
    filterValues.value[f.field] = f.operator === "like" ? f.value.replace(/%/g, "") : f.value;
  }
  // Apply sort from view
  if (view.sort_field) sortField.value = view.sort_field;
  if (view.sort_order) sortOrder.value = view.sort_order as "asc" | "desc";
  page.value = 0;
  loadData();
}

function clearView() {
  setActiveView("");
  clearAllFilters();
}

function saveCurrentView() {
  if (!newViewName.value.trim()) return;
  // If marking as default, unset default on all other views
  if (newViewDefault.value) {
    for (const v of views.value) {
      v.is_default = false;
    }
  }
  const view: SavedView = {
    name: newViewName.value.trim(),
    is_default: newViewDefault.value,
    filters: activeFilters.value.filter((f) => f.value).map((f) => [f.field, f.operator, f.value]),
    columns: fieldNames.value,
    sort_field: sortField.value,
    sort_order: sortOrder.value,
  };
  addView(view);
  showSaveView.value = false;
  newViewName.value = "";
  newViewDefault.value = false;
}

// Update the currently active view — opens edit modal with current filters baked in
function updateCurrentView() {
  if (!activeView.value) return;
  startEditView(activeView.value);
}

// Edit view: open modal pre-filled with the view's name/default, allow rename
const editingViewName = ref("");
const showEditView = ref(false);
const editViewName = ref("");
const editViewDefault = ref(false);

function startEditView(viewName: string) {
  const view = views.value.find((v) => v.name === viewName);
  if (!view) return;
  editingViewName.value = viewName;
  editViewName.value = view.name;
  editViewDefault.value = view.is_default || false;
  showEditView.value = true;
}

function saveEditView() {
  if (!editViewName.value.trim()) return;
  const view = views.value.find((v) => v.name === editingViewName.value);
  if (!view) return;

  // If marking as default, unset others
  if (editViewDefault.value) {
    for (const v of views.value) {
      v.is_default = v.name === editingViewName.value ? true : false;
    }
  } else {
    view.is_default = false;
  }

  // Update filters and sort with current state
  if (hasActiveFilters.value) {
    view.filters = activeFilters.value.filter((f) => f.value).map((f) => [f.field, f.operator, f.value]);
    view.columns = fieldNames.value;
  }
  view.sort_field = sortField.value;
  view.sort_order = sortOrder.value;

  // Rename
  const oldName = view.name;
  view.name = editViewName.value.trim();

  if (activeView.value === oldName) {
    activeView.value = view.name;
  }

  saveViews();
  showEditView.value = false;
}

function deleteViewFromEdit() {
  removeView(editingViewName.value);
  showEditView.value = false;
}

function toggleDefault(viewName: string) {
  const wasDefault = views.value.find((v) => v.name === viewName)?.is_default;
  for (const v of views.value) {
    v.is_default = v.name === viewName ? !wasDefault : false;
  }
  saveViews();
}

// --- Init ---
watch(
  () => props.doctype,
  async () => {
    page.value = 0;
    activeFilters.value = [];
    filterValues.value = {};
    extraFilterFieldnames.value = [];
    clearSelection();
    await loadMeta();
    // Init sort from meta
    sortField.value = meta.value?.sort_field || "modified";
    sortOrder.value = (meta.value?.sort_order as "asc" | "desc") || "desc";
    await loadViews();
    const defaultView = views.value.find((v) => v.is_default);
    if (defaultView) {
      applyView(defaultView);
    } else {
      await loadData();
    }
  },
  { immediate: true },
);

watch(page, () => { clearSelection(); loadData(); });

// Close add-filter dropdown on outside click
function onDocClick() {
  showAddFilter.value = false;
}

function openDoc(id: string) {
  router.push(`/app/${props.doctype}/${id}`);
}

function formatValue(value: unknown, field: DocFieldMeta): string {
  if (value === null || value === undefined) return "\u2014";
  if (field.fieldtype === "Check") return value ? "Yes" : "No";
  if (field.fieldtype === "Currency" || field.fieldtype === "Float") return Number(value).toLocaleString();
  return String(value);
}

function statusColor(value: unknown): "green" | "blue" | "red" | "amber" | "gray" {
  const v = String(value).toLowerCase();
  if (["open", "active", "enabled", "yes", "approved"].includes(v)) return "green";
  if (["closed", "completed", "done", "resolved"].includes(v)) return "blue";
  if (["cancelled", "rejected", "disabled", "no"].includes(v)) return "red";
  if (["pending", "draft", "on hold"].includes(v)) return "amber";
  return "gray";
}

function isStatusField(field: DocFieldMeta): boolean {
  return field.fieldtype === "Select" && (field.fieldname === "status" || field.fieldname.endsWith("_status"));
}

function fieldLabel(fieldname: string): string {
  const f = allDataFields.value.find((fd) => fd.fieldname === fieldname);
  return f?.label || fieldname;
}

// Realtime: auto-refresh list when any doc of this type changes
function onDocUpdate(data: unknown) {
  const d = data as { doctype?: string };
  if (d.doctype === props.doctype) {
    loadData();
  }
}
onMounted(() => socket.on("doc_update", onDocUpdate));
onUnmounted(() => socket.off("doc_update", onDocUpdate));
</script>

<template>
  <div class="h-full flex flex-col" @click="onDocClick">
    <!-- Header -->
    <LPageHeader :title="doctype" :subtitle="!loading ? `${rows.length}${rows.length >= pageSize ? '+' : ''} records` : undefined">
      <template #actions>
        <!-- Custom buttons from client script -->
        <LButton
          v-for="(btn, i) in customButtons"
          :key="'cb-' + i"
          :variant="(btn.variant as any) || 'secondary'"
          size="sm"
          @click="runCustomButton(btn)"
        >{{ btn.label }}</LButton>

        <template v-if="isAdmin() && doctype !== 'DocType'">
          <LButton variant="secondary" @click="router.push(`/app/DocType/${doctype}`)">Edit DocType</LButton>
          <LButton variant="secondary" @click="router.push(`/app/customize-form/${doctype}`)">Customize</LButton>
        </template>
        <LButton @click="router.push(`/app/${doctype}/new`)">
          <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          Add
        </LButton>
      </template>
    </LPageHeader>

    <!-- Filter bar -->
    <div class="px-6 pt-3">
      <div class="flex items-end gap-2 flex-wrap">
        <!-- Each visible filter -->
        <template v-for="field in visibleFilterFields" :key="field.fieldname">
          <div class="flex flex-col relative group">
            <div class="flex items-center gap-0.5 mb-0.5">
              <label class="text-[10px] font-medium text-text-light">{{ field.label || field.fieldname }}</label>
              <!-- Remove button for extra (non-standard) filters -->
              <button
                v-if="!field.in_standard_filter"
                class="text-text-light hover:text-danger opacity-0 group-hover:opacity-100 transition-opacity"
                @click="removeFilterField(field.fieldname)"
                title="Remove filter"
              >
                <svg class="w-2.5 h-2.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
              </button>
            </div>
            <!-- Select -->
            <select
              v-if="field.fieldtype === 'Select'"
              :value="filterValues[field.fieldname] || ''"
              class="h-7 px-2 text-[12px] border border-border rounded-md bg-white text-text focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 min-w-[100px]"
              @change="onFilterChange(field.fieldname, ($event.target as HTMLSelectElement).value)"
            >
              <option value="">All</option>
              <option v-for="opt in (field.options || '').split('\n').filter((o: string) => o.trim())" :key="opt" :value="opt.trim()">{{ opt.trim() }}</option>
            </select>
            <!-- Check -->
            <select
              v-else-if="field.fieldtype === 'Check'"
              :value="filterValues[field.fieldname] || ''"
              class="h-7 px-2 text-[12px] border border-border rounded-md bg-white text-text focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 min-w-[80px]"
              @change="onFilterChange(field.fieldname, ($event.target as HTMLSelectElement).value)"
            >
              <option value="">All</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
            <!-- Date -->
            <input
              v-else-if="field.fieldtype === 'Date' || field.fieldtype === 'Datetime'"
              type="date"
              :value="filterValues[field.fieldname] || ''"
              class="h-7 px-2 text-[12px] border border-border rounded-md bg-white text-text focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400"
              @change="onFilterChange(field.fieldname, ($event.target as HTMLInputElement).value)"
            />
            <!-- Text / Link / default -->
            <input
              v-else
              type="text"
              :value="filterValues[field.fieldname] || ''"
              :placeholder="field.label || field.fieldname"
              class="h-7 px-2 text-[12px] border border-border rounded-md bg-white text-text placeholder-text-light focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 w-[140px]"
              @input="onFilterChange(field.fieldname, ($event.target as HTMLInputElement).value)"
            />
          </div>
        </template>

        <!-- Add Filter button + dropdown -->
        <div class="flex flex-col relative">
          <span class="text-[10px] mb-0.5">&nbsp;</span>
          <button
            class="h-7 px-2.5 text-[12px] font-medium text-primary-600 border border-dashed border-primary-300 rounded-md hover:bg-primary-50 transition-colors flex items-center gap-1"
            @click.stop="showAddFilter = !showAddFilter"
          >
            <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
            Filter
          </button>
          <!-- Dropdown -->
          <div
            v-if="showAddFilter && addableFields.length > 0"
            class="absolute top-full left-0 mt-1 z-30 bg-white border border-border rounded-lg shadow-lg py-1 min-w-[180px] max-h-[240px] overflow-y-auto"
            @click.stop
          >
            <button
              v-for="f in addableFields"
              :key="f.fieldname"
              class="w-full text-left px-3 py-1.5 text-[12px] text-text hover:bg-surface-muted transition-colors flex items-center justify-between"
              @click="addFilterField(f.fieldname)"
            >
              <span>{{ f.label || f.fieldname }}</span>
              <span class="text-[10px] text-text-light">{{ f.fieldtype }}</span>
            </button>
            <div v-if="addableFields.length === 0" class="px-3 py-2 text-[11px] text-text-light">No more fields</div>
          </div>
        </div>

        <!-- Clear all -->
        <div v-if="hasActiveFilters" class="flex flex-col">
          <span class="text-[10px] mb-0.5">&nbsp;</span>
          <button
            class="h-7 px-2 text-[11px] text-text-muted hover:text-danger transition-colors"
            @click="clearAllFilters"
          >Clear all</button>
        </div>

        <div class="flex-1" />

        <!-- Save view -->
        <div v-if="hasActiveFilters" class="flex flex-col">
          <span class="text-[10px] mb-0.5">&nbsp;</span>
          <button
            class="h-7 px-2 text-[12px] text-primary-600 hover:text-primary-700 font-medium"
            @click="showSaveView = true"
          >Save View</button>
        </div>
      </div>
    </div>

    <!-- Saved view tabs -->
    <div v-if="views.length > 0" class="px-6 pt-2 flex items-center gap-1 flex-wrap">
      <button
        :class="[
          'px-2.5 py-1 text-[12px] rounded-md transition-colors',
          !activeView ? 'bg-primary-50 text-primary-700 font-medium' : 'text-text-muted hover:bg-surface-raised',
        ]"
        @click="clearView"
      >All</button>
      <button
        v-for="view in views"
        :key="view.name"
        :class="[
          'px-2.5 py-1 text-[12px] rounded-md transition-colors group flex items-center gap-1',
          activeView === view.name ? 'bg-primary-50 text-primary-700 font-medium' : 'text-text-muted hover:bg-surface-raised',
        ]"
        @click="applyView(view)"
      >
        <span
          :class="[
            'text-[10px] cursor-pointer transition-colors',
            view.is_default ? 'text-amber-500' : 'text-text-light/30 hover:text-amber-400 opacity-0 group-hover:opacity-100',
          ]"
          @click.stop="toggleDefault(view.name)"
          :title="view.is_default ? 'Remove as default' : 'Set as default'"
        >&#9733;</span>
        {{ view.name }}
        <span
          class="text-text-light hover:text-text opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
          @click.stop="startEditView(view.name)"
          title="Edit view"
        >
          <svg class="w-2.5 h-2.5 inline" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Z" /></svg>
        </span>
        <span
          class="text-text-light hover:text-danger opacity-0 group-hover:opacity-100 transition-opacity"
          @click.stop="removeView(view.name)"
        >&times;</span>
      </button>
      <!-- Update view button (when active view has changed filters) -->
      <button
        v-if="activeView && hasActiveFilters"
        class="px-2 py-1 text-[11px] text-primary-600 hover:text-primary-700 font-medium"
        @click="updateCurrentView"
        title="Update this view with current filters"
      >Update View</button>
    </div>

    <!-- Active filter pills -->
    <div v-if="activeFilters.length > 0" class="px-6 pt-2 flex items-center gap-1.5 flex-wrap">
      <span
        v-for="(filter, fi) in activeFilters"
        :key="fi"
        class="inline-flex items-center gap-1 px-2 py-0.5 bg-primary-50 text-primary-700 text-[11px] rounded-md"
      >
        {{ fieldLabel(filter.field) }} {{ filter.operator }} {{ filter.operator === 'like' ? filter.value.replace(/%/g, '') : filter.value }}
        <button class="hover:text-danger" @click="removeFilter(fi)">&times;</button>
      </span>
    </div>

    <!-- Error -->
    <div v-if="error" class="mx-6 mt-3">
      <LAlert type="error">{{ error }}</LAlert>
    </div>

    <!-- Bulk action bar -->
    <div
      v-if="selectedIds.size > 0"
      class="mx-6 mt-3 flex items-center gap-3 px-4 py-2 bg-primary-50 border border-primary-200 rounded-lg"
    >
      <span class="text-[12px] font-medium text-primary-700">{{ selectedIds.size }} selected</span>
      <LButton variant="danger" size="sm" @click="bulkDelete">Delete</LButton>
      <LButton variant="secondary" size="sm" @click="exportCsv">Export CSV</LButton>
      <button class="text-[11px] text-text-muted hover:text-text ml-auto" @click="clearSelection">Clear selection</button>
    </div>

    <!-- Table -->
    <div class="flex-1 overflow-auto px-6 py-4">
      <div class="bg-white border border-border rounded-lg overflow-hidden">
        <table class="w-full">
          <thead>
            <tr class="border-b border-border">
              <th class="w-8 px-3 py-2 bg-surface-muted/50">
                <input
                  type="checkbox"
                  :checked="selectAll"
                  class="w-3.5 h-3.5 rounded border-border-strong text-primary-600 focus:ring-primary-500"
                  @change="toggleSelectAll"
                />
              </th>
              <th
                class="text-left px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 cursor-pointer select-none hover:text-text transition-colors"
                @click="toggleSort('id')"
              >
                <span class="inline-flex items-center gap-1">
                  ID
                  <svg v-if="sortField === 'id'" class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path v-if="sortOrder === 'asc'" stroke-linecap="round" stroke-linejoin="round" d="M4.5 15.75l7.5-7.5 7.5 7.5" />
                    <path v-else stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                  </svg>
                </span>
              </th>
              <th
                v-for="col in listColumns"
                :key="col.fieldname"
                class="text-left px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 cursor-pointer select-none hover:text-text transition-colors"
                @click="toggleSort(col.fieldname)"
              >
                <span class="inline-flex items-center gap-1">
                  {{ col.label || col.fieldname }}
                  <svg v-if="sortField === col.fieldname" class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path v-if="sortOrder === 'asc'" stroke-linecap="round" stroke-linejoin="round" d="M4.5 15.75l7.5-7.5 7.5 7.5" />
                    <path v-else stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                  </svg>
                </span>
              </th>
            </tr>
          </thead>
          <tbody class="text-[13px]">
            <!-- Loading -->
            <template v-if="loading">
              <tr v-for="i in 5" :key="i" class="border-b border-border last:border-0">
                <td class="px-3 py-2.5"><div class="h-3 w-3.5 bg-surface-raised rounded animate-pulse" /></td>
                <td class="px-3 py-2.5"><div class="h-3 w-14 bg-surface-raised rounded animate-pulse" /></td>
                <td v-for="col in listColumns" :key="col.fieldname" class="px-3 py-2.5">
                  <div class="h-3 rounded bg-surface-raised animate-pulse" :style="{ width: `${50 + Math.random() * 60}px` }" />
                </td>
              </tr>
            </template>

            <!-- Empty -->
            <tr v-else-if="rows.length === 0">
              <td :colspan="listColumns.length + 2" class="px-3 py-12 text-center">
                <p class="text-[13px] text-text-muted">{{ activeFilters.length > 0 ? 'No matching records' : 'No records yet' }}</p>
                <p class="text-[12px] text-text-light mt-0.5">{{ activeFilters.length > 0 ? 'Try different filters' : `Create your first ${doctype}` }}</p>
              </td>
            </tr>

            <!-- Rows -->
            <tr
              v-else
              v-for="row in rows"
              :key="String(row.id)"
              :class="[
                'border-b border-border last:border-0 hover:bg-surface-muted/40 cursor-pointer transition-colors',
                selectedIds.has(String(row.id)) ? 'bg-primary-50/50' : '',
              ]"
              @click="openDoc(String(row.id))"
            >
              <td class="px-3 py-2.5" @click.stop>
                <input
                  type="checkbox"
                  :checked="selectedIds.has(String(row.id))"
                  class="w-3.5 h-3.5 rounded border-border-strong text-primary-600 focus:ring-primary-500"
                  @change="toggleSelect(String(row.id))"
                />
              </td>
              <td class="px-3 py-2.5 font-medium text-primary-600 whitespace-nowrap">{{ row.id }}</td>
              <td v-for="col in listColumns" :key="col.fieldname" class="px-3 py-2.5">
                <LBadge
                  v-if="isStatusField(col) && row[col.fieldname]"
                  :color="statusColor(row[col.fieldname])"
                  :label="String(row[col.fieldname])"
                />
                <span v-else class="text-text">{{ formatValue(row[col.fieldname], col) }}</span>
              </td>
            </tr>
          </tbody>
        </table>

        <!-- Pagination -->
        <div
          v-if="rows.length >= pageSize || page > 0"
          class="flex items-center justify-between px-3 py-2 border-t border-border bg-surface-muted/30"
        >
          <LButton variant="secondary" size="sm" :disabled="page === 0" @click="page--">Prev</LButton>
          <span class="text-[11px] text-text-muted">Page {{ page + 1 }}</span>
          <LButton variant="secondary" size="sm" :disabled="rows.length < pageSize" @click="page++">Next</LButton>
        </div>
      </div>
    </div>

    <!-- Save View Modal -->
    <LModal :open="showSaveView" title="Save View" @close="showSaveView = false">
      <div class="mt-2 space-y-3">
        <div>
          <label class="block text-[12px] font-medium text-text-muted mb-1">View Name</label>
          <LInput v-model="newViewName" placeholder="e.g. My Open Items" />
        </div>
        <label class="flex items-center gap-2 text-[13px] text-text cursor-pointer select-none">
          <input v-model="newViewDefault" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" />
          Load this view by default
        </label>
      </div>
      <template #footer>
        <LButton variant="secondary" @click="showSaveView = false">Cancel</LButton>
        <LButton :disabled="!newViewName.trim()" @click="saveCurrentView">Save</LButton>
      </template>
    </LModal>

    <!-- Edit View Modal -->
    <LModal :open="showEditView" title="Edit View" @close="showEditView = false">
      <div class="mt-2 space-y-3">
        <div>
          <label class="block text-[12px] font-medium text-text-muted mb-1">View Name</label>
          <LInput v-model="editViewName" placeholder="View name" />
        </div>
        <label class="flex items-center gap-2 text-[13px] text-text cursor-pointer select-none">
          <input v-model="editViewDefault" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" />
          Load this view by default
        </label>
      </div>
      <template #footer>
        <LButton variant="danger" size="sm" @click="deleteViewFromEdit">Delete</LButton>
        <div class="flex-1" />
        <LButton variant="secondary" @click="showEditView = false">Cancel</LButton>
        <LButton :disabled="!editViewName.trim()" @click="saveEditView">Save</LButton>
      </template>
    </LModal>
  </div>
</template>
