<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { loom, type DocFieldMeta, type DocPermMeta, LoomApiError } from "@/utils/call";
import { useDoctypeList } from "@/composables/useDoctypeList";
import { LButton, LAlert, LPageHeader } from "@/components/ui";

const props = defineProps<{
  doctype?: string;
}>();

const router = useRouter();
const { refresh: refreshDoctypeList } = useDoctypeList();

// --- Section-based field model ---
interface BuilderField {
  fieldname: string;
  label: string;
  fieldtype: string;
  options?: string;
  reqd?: boolean;
  unique?: boolean;
  read_only?: boolean;
  hidden?: boolean;
  in_list_view?: boolean;
  in_standard_filter?: boolean;
  description?: string;
  permlevel?: number;
  fetch_from?: string;
  depends_on?: string;
  default?: unknown;
}

interface BuilderSection {
  label: string;
  collapsible: boolean;
  columns: number; // 1, 2, or 3
  fields: BuilderField[][]; // fields[colIdx] = fields in that column
}

const isEditMode = computed(() => !!props.doctype);
const loading = ref(false);
const name = ref("");
const module = ref("Core");
const namingRule = ref("autoincrement");
const autoname = ref("");
const isSubmittable = ref(false);
const isChildTable = ref(false);
const sections = ref<BuilderSection[]>([
  { label: "", collapsible: false, columns: 1, fields: [[]] },
]);
const permissions = ref<DocPermMeta[]>([
  { role: "Administrator", permlevel: 0, read: true, write: true, create: true, delete: true, submit: false, cancel: false },
]);
const saving = ref(false);
const activeTab = ref<"settings" | "fields" | "permissions">("settings");
const error = ref("");

// Editing state for field detail panel
const editingField = ref<{ sectionIdx: number; colIdx: number; fieldIdx: number } | null>(null);
const editingFieldData = computed(() => {
  if (!editingField.value) return null;
  const { sectionIdx, colIdx, fieldIdx } = editingField.value;
  return sections.value[sectionIdx]?.fields[colIdx]?.[fieldIdx] ?? null;
});

const availableRoles = ["Administrator", "System Manager", "All", "Guest"];

// --- Parse flat fields into sections ---
function parseSections(flatFields: DocFieldMeta[]): BuilderSection[] {
  const result: BuilderSection[] = [];
  let current: BuilderSection = { label: "", collapsible: false, columns: 1, fields: [[]] };
  result.push(current);

  for (const f of flatFields) {
    if (f.fieldtype === "SectionBreak" || f.fieldtype === "TabBreak") {
      current = { label: f.label || "", collapsible: f.collapsible || false, columns: 1, fields: [[]] };
      result.push(current);
    } else if (f.fieldtype === "ColumnBreak") {
      current.columns++;
      current.fields.push([]);
    } else {
      current.fields[current.fields.length - 1].push({
        fieldname: f.fieldname,
        label: f.label || "",
        fieldtype: f.fieldtype,
        options: f.options,
        reqd: f.reqd,
        unique: f.unique,
        read_only: f.read_only,
        hidden: f.hidden,
        in_list_view: f.in_list_view,
        in_standard_filter: f.in_standard_filter,
        description: f.description,
        permlevel: f.permlevel,
        fetch_from: f.fetch_from,
        depends_on: f.depends_on,
        default: f.default,
      });
    }
  }

  // Remove empty leading section if nothing in it
  if (result.length > 1 && result[0].fields.every((col) => col.length === 0) && !result[0].label) {
    result.shift();
  }

  return result.length > 0 ? result : [{ label: "", collapsible: false, columns: 1, fields: [[]] }];
}

// --- Flatten sections back to flat fields ---
function flattenToFields(): DocFieldMeta[] {
  const flat: DocFieldMeta[] = [];

  for (let si = 0; si < sections.value.length; si++) {
    const section = sections.value[si];

    // Add SectionBreak (skip for first section with no label)
    if (si > 0 || section.label) {
      flat.push({
        fieldname: `sb_${si}`,
        label: section.label,
        fieldtype: "SectionBreak",
        collapsible: section.collapsible,
      } as DocFieldMeta);
    }

    for (let ci = 0; ci < section.columns; ci++) {
      // Add ColumnBreak before 2nd+ columns
      if (ci > 0) {
        flat.push({
          fieldname: `cb_${si}_${ci}`,
          label: "",
          fieldtype: "ColumnBreak",
        } as DocFieldMeta);
      }

      const colFields = section.fields[ci] || [];
      for (const f of colFields) {
        flat.push(f as DocFieldMeta);
      }
    }
  }

  return flat;
}

onMounted(async () => {
  if (props.doctype) {
    loading.value = true;
    try {
      const res = await loom.getMeta(props.doctype);
      const meta = res.data;
      name.value = meta.name;
      module.value = meta.module || "Core";
      namingRule.value = meta.naming_rule || "autoincrement";
      autoname.value = meta.autoname || "";
      isSubmittable.value = meta.is_submittable || false;
      isChildTable.value = meta.is_child_table || false;
      sections.value = parseSections(meta.fields);
      if (meta.permissions && meta.permissions.length > 0) {
        permissions.value = meta.permissions.map((p) => ({ ...p }));
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : "Failed to load DocType";
    } finally {
      loading.value = false;
    }
  }
});

const dataFieldTypes = [
  { value: "Data", group: "Text" },
  { value: "Text", group: "Text" },
  { value: "SmallText", group: "Text" },
  { value: "LongText", group: "Text" },
  { value: "Code", group: "Text" },
  { value: "TextEditor", group: "Text" },
  { value: "Password", group: "Text" },
  { value: "Int", group: "Number" },
  { value: "Float", group: "Number" },
  { value: "Currency", group: "Number" },
  { value: "Percent", group: "Number" },
  { value: "Check", group: "Choice" },
  { value: "Select", group: "Choice" },
  { value: "Link", group: "Link" },
  { value: "Date", group: "Date" },
  { value: "Datetime", group: "Date" },
  { value: "Time", group: "Date" },
  { value: "Attach", group: "File" },
  { value: "AttachImage", group: "File" },
  { value: "Table", group: "Layout" },
  { value: "Color", group: "Other" },
  { value: "JSON", group: "Other" },
];

const namingRules = [
  { value: "autoincrement", label: "Auto Increment" },
  { value: "hash", label: "Random Hash" },
  { value: "by_fieldname", label: "By Field" },
  { value: "series", label: "Naming Series" },
  { value: "prompt", label: "Prompt" },
  { value: "expression", label: "Expression" },
];

const needsAutoname = computed(() => ["by_fieldname", "series", "expression"].includes(namingRule.value));
const autonameHint = computed(() => {
  switch (namingRule.value) {
    case "by_fieldname": return "e.g. employee_name";
    case "series": return "e.g. HR-EMP-.YYYY.-.#####";
    case "expression": return "e.g. {employee_name}-{department}";
    default: return "";
  }
});
const autonameLabel = computed(() => {
  switch (namingRule.value) {
    case "by_fieldname": return "Field Name";
    case "series": return "Naming Series";
    case "expression": return "Expression";
    default: return "Autoname";
  }
});

// --- Section operations ---
function addSection() {
  sections.value.push({ label: "", collapsible: false, columns: 1, fields: [[]] });
}

function removeSection(si: number) {
  if (sections.value.length <= 1) return;
  sections.value.splice(si, 1);
  editingField.value = null;
}

function moveSectionUp(si: number) {
  if (si <= 0) return;
  const temp = sections.value[si];
  sections.value[si] = sections.value[si - 1];
  sections.value[si - 1] = temp;
}

function moveSectionDown(si: number) {
  if (si >= sections.value.length - 1) return;
  const temp = sections.value[si];
  sections.value[si] = sections.value[si + 1];
  sections.value[si + 1] = temp;
}

function setColumnCount(si: number, count: number) {
  const section = sections.value[si];
  const old = section.columns;
  section.columns = count;

  // Grow: add empty columns
  while (section.fields.length < count) {
    section.fields.push([]);
  }

  // Shrink: move fields from removed columns to last remaining column
  if (count < old) {
    const overflow: BuilderField[] = [];
    while (section.fields.length > count) {
      overflow.push(...section.fields.pop()!);
    }
    if (overflow.length > 0) {
      section.fields[count - 1].push(...overflow);
    }
  }
}

// --- Field operations ---
function addFieldToColumn(si: number, ci: number) {
  sections.value[si].fields[ci].push({
    fieldname: "",
    label: "",
    fieldtype: "Data",
    reqd: false,
    in_list_view: false,
    in_standard_filter: false,
  });
  const fieldIdx = sections.value[si].fields[ci].length - 1;
  editingField.value = { sectionIdx: si, colIdx: ci, fieldIdx };
}

function removeField(si: number, ci: number, fi: number) {
  sections.value[si].fields[ci].splice(fi, 1);
  if (editingField.value?.sectionIdx === si && editingField.value?.colIdx === ci && editingField.value?.fieldIdx === fi) {
    editingField.value = null;
  }
}


function autoFieldname(field: BuilderField) {
  if (field.label && !field.fieldname) {
    field.fieldname = field.label
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_|_$/g, "");
  }
}

function selectField(si: number, ci: number, fi: number) {
  editingField.value = { sectionIdx: si, colIdx: ci, fieldIdx: fi };
}

// --- Drag and drop ---
const dragSource = ref<{ si: number; ci: number; fi: number } | null>(null);
const dropTarget = ref<{ si: number; ci: number; fi: number } | null>(null);

function onDragStart(e: DragEvent, si: number, ci: number, fi: number) {
  dragSource.value = { si, ci, fi };
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", ""); // required for Firefox
  }
  // Make the dragged element semi-transparent
  (e.target as HTMLElement).style.opacity = "0.4";
}

function onDragEnd(e: DragEvent) {
  (e.target as HTMLElement).style.opacity = "1";
  dragSource.value = null;
  dropTarget.value = null;
}

function onDragOverField(e: DragEvent, si: number, ci: number, fi: number) {
  if (!dragSource.value) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  dropTarget.value = { si, ci, fi };
}

function onDragOverEmpty(e: DragEvent, si: number, ci: number) {
  if (!dragSource.value) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  const colFields = sections.value[si].fields[ci] || [];
  dropTarget.value = { si, ci, fi: colFields.length };
}

function onDrop(e: DragEvent, targetSi: number, targetCi: number, targetFi: number) {
  e.preventDefault();
  if (!dragSource.value) return;

  const src = dragSource.value;

  // Don't drop on self at same position
  if (src.si === targetSi && src.ci === targetCi && src.fi === targetFi) {
    dragSource.value = null;
    dropTarget.value = null;
    return;
  }

  // Remove field from source
  const [field] = sections.value[src.si].fields[src.ci].splice(src.fi, 1);

  // Adjust target index if removing from the same column above the target
  let adjustedFi = targetFi;
  if (src.si === targetSi && src.ci === targetCi && src.fi < targetFi) {
    adjustedFi--;
  }

  // Insert at target
  sections.value[targetSi].fields[targetCi].splice(adjustedFi, 0, field);

  // Update editing selection to follow the moved field
  if (editingField.value &&
      editingField.value.sectionIdx === src.si &&
      editingField.value.colIdx === src.ci &&
      editingField.value.fieldIdx === src.fi) {
    editingField.value = { sectionIdx: targetSi, colIdx: targetCi, fieldIdx: adjustedFi };
  }

  dragSource.value = null;
  dropTarget.value = null;
}

function isDropIndicator(si: number, ci: number, fi: number): boolean {
  if (!dropTarget.value || !dragSource.value) return false;
  return dropTarget.value.si === si && dropTarget.value.ci === ci && dropTarget.value.fi === fi;
}

// --- Total data field count for validation ---
const totalFieldCount = computed(() => {
  let count = 0;
  for (const s of sections.value) {
    for (const col of s.fields) {
      count += col.length;
    }
  }
  return count;
});

const isValid = computed(() => {
  if (!name.value.trim()) return false;
  if (totalFieldCount.value === 0) return false;
  for (const s of sections.value) {
    for (const col of s.fields) {
      for (const f of col) {
        if (!f.fieldname || !f.fieldtype) return false;
      }
    }
  }
  return true;
});

function buildDoctypeJson() {
  const flatFields = flattenToFields();
  return {
    name: name.value,
    module: module.value,
    naming_rule: isChildTable.value ? "autoincrement" : namingRule.value,
    autoname: isChildTable.value ? undefined : (needsAutoname.value ? autoname.value : undefined),
    is_submittable: isChildTable.value ? false : isSubmittable.value,
    is_child_table: isChildTable.value,
    is_single: false,
    is_virtual: false,
    is_tree: false,
    fields: flatFields.map((f) => ({
      fieldname: f.fieldname,
      label: f.label || f.fieldname,
      fieldtype: f.fieldtype,
      options: f.options || undefined,
      reqd: f.reqd || false,
      unique: f.unique || false,
      read_only: f.read_only || false,
      hidden: f.hidden || false,
      in_list_view: f.in_list_view || false,
      in_standard_filter: f.in_standard_filter || false,
      description: f.description || undefined,
      permlevel: f.permlevel || 0,
      collapsible: (f as any).collapsible || undefined,
      fetch_from: f.fetch_from || undefined,
      depends_on: f.depends_on || undefined,
      default: f.default || undefined,
    })),
    permissions: permissions.value.map((p) => ({
      role: p.role,
      permlevel: p.permlevel || 0,
      read: p.read || false,
      write: p.write || false,
      create: p.create || false,
      delete: p.delete || false,
      submit: isSubmittable.value ? (p.submit || false) : false,
      cancel: isSubmittable.value ? (p.cancel || false) : false,
    })),
  };
}

async function save() {
  if (!isValid.value) return;
  saving.value = true;
  error.value = "";
  try {
    const doctypeJson = buildDoctypeJson();
    if (isEditMode.value) {
      await loom.resource("DocType").update(props.doctype!, doctypeJson);
    } else {
      await loom.resource("DocType").insert(doctypeJson);
    }
    await refreshDoctypeList();
    router.push(`/app/${name.value}`);
  } catch (e: unknown) {
    error.value = e instanceof LoomApiError ? e.message : (e instanceof Error ? e.message : "Failed to save DocType");
  } finally {
    saving.value = false;
  }
}

function addPermission() {
  permissions.value.push({ role: "All", permlevel: 0, read: true, write: false, create: false, delete: false, submit: false, cancel: false });
}

function removePermission(index: number) {
  permissions.value.splice(index, 1);
}
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <LPageHeader :title="doctype ? `Edit ${doctype}` : 'New DocType'">
      <template #breadcrumb>
        <button
          class="inline-flex items-center gap-0.5 text-[12px] text-text-muted hover:text-primary-600 transition-colors"
          @click="router.push('/app/DocType')"
        >
          <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5" />
          </svg>
          DocType
        </button>
      </template>
      <template #actions>
        <LButton :disabled="saving || !isValid" :loading="saving" @click="save">
          {{ isEditMode ? "Save Changes" : "Create DocType" }}
        </LButton>
      </template>
    </LPageHeader>

    <!-- Tabs -->
    <div class="border-b border-border bg-white px-6 shrink-0">
      <div class="flex gap-0">
        <button
          v-for="tab in [
            { key: 'settings', label: 'Settings' },
            { key: 'fields', label: `Fields${totalFieldCount > 0 ? ` (${totalFieldCount})` : ''}` },
            { key: 'permissions', label: `Permissions${permissions.length > 0 ? ` (${permissions.length})` : ''}` },
          ]"
          :key="tab.key"
          :class="[
            'px-4 py-2.5 text-[13px] font-medium border-b-2 transition-colors -mb-px',
            activeTab === tab.key
              ? 'border-primary-600 text-primary-700'
              : 'border-transparent text-text-muted hover:text-text hover:border-border',
          ]"
          @click="activeTab = tab.key as any"
        >{{ tab.label }}</button>
      </div>
    </div>

    <div class="flex-1 overflow-auto">
      <div class="flex">
        <!-- Main area -->
        <div class="flex-1 p-6 space-y-5 min-w-0">
          <!-- Error -->
          <LAlert v-if="error" type="error" dismissible @dismiss="error = ''">{{ error }}</LAlert>

          <!-- Settings -->
          <div v-show="activeTab === 'settings'" class="bg-surface border border-border rounded-xl overflow-hidden">
            <div class="px-5 py-3 border-b border-border bg-surface-muted/30">
              <h3 class="text-[13px] font-semibold text-text-muted">Settings</h3>
            </div>
            <div class="p-5">
              <div class="grid grid-cols-2 gap-5">
                <div>
                  <label class="block text-[13px] font-medium text-text mb-1.5">Name <span class="text-danger">*</span></label>
                  <input v-model="name" type="text" placeholder="e.g. Employee, Invoice" :disabled="isEditMode" class="w-full px-3 py-[7px] text-sm border border-border rounded-lg bg-surface text-text placeholder-text-light focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 disabled:bg-surface-raised disabled:text-text-muted disabled:cursor-not-allowed transition-colors" />
                </div>
                <div>
                  <label class="block text-[13px] font-medium text-text mb-1.5">Module</label>
                  <input v-model="module" type="text" placeholder="e.g. HR, Accounting" class="w-full px-3 py-[7px] text-sm border border-border rounded-lg bg-surface text-text placeholder-text-light focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-colors" />
                </div>
                <div v-if="!isChildTable">
                  <label class="block text-[13px] font-medium text-text mb-1.5">Naming Rule</label>
                  <div class="relative">
                    <select v-model="namingRule" class="w-full px-3 py-[7px] text-sm border border-border rounded-lg bg-surface text-text appearance-none pr-8 focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-colors">
                      <option v-for="nr in namingRules" :key="nr.value" :value="nr.value">{{ nr.label }}</option>
                    </select>
                    <svg class="absolute right-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-text-light pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
                  </div>
                </div>
                <div v-if="!isChildTable && needsAutoname">
                  <label class="block text-[13px] font-medium text-text mb-1.5">{{ autonameLabel }} <span class="text-danger">*</span></label>
                  <input v-model="autoname" type="text" :placeholder="autonameHint" class="w-full px-3 py-[7px] text-sm border border-border rounded-lg bg-surface text-text placeholder-text-light focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-colors font-mono" />
                  <p class="mt-1 text-[11px] text-text-light">{{ autonameHint }}</p>
                </div>
                <div v-else-if="!isChildTable" class="flex items-end pb-1" />
                <div class="flex items-end pb-1">
                  <label class="inline-flex items-center gap-2.5 cursor-pointer select-none">
                    <input v-model="isChildTable" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" />
                    <span class="text-sm text-text">Child Table</span>
                  </label>
                </div>
                <div v-if="!isChildTable" class="flex items-end pb-1">
                  <label class="inline-flex items-center gap-2.5 cursor-pointer select-none">
                    <input v-model="isSubmittable" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" />
                    <span class="text-sm text-text">Submittable</span>
                  </label>
                </div>
              </div>
            </div>
          </div>

          <!-- Fields — Section-based layout -->
          <div v-show="activeTab === 'fields'" class="space-y-3">
            <div class="flex items-center justify-between">
              <h3 class="text-[13px] font-semibold text-text-muted">
                Fields
                <span v-if="totalFieldCount > 0" class="text-text-light font-normal">({{ totalFieldCount }})</span>
              </h3>
              <LButton variant="secondary" size="sm" @click="addSection">
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
                Add Section
              </LButton>
            </div>

            <!-- Each section -->
            <div
              v-for="(section, si) in sections"
              :key="si"
              class="bg-surface border border-border rounded-xl overflow-hidden"
            >
              <!-- Section header -->
              <div class="px-4 py-2.5 border-b border-border bg-surface-muted/30 flex items-center gap-2">
                <!-- Reorder -->
                <div class="flex gap-0.5 shrink-0">
                  <button class="p-0.5 text-text-light hover:text-text rounded disabled:opacity-20" :disabled="si === 0" @click="moveSectionUp(si)" title="Move up">
                    <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" /></svg>
                  </button>
                  <button class="p-0.5 text-text-light hover:text-text rounded disabled:opacity-20" :disabled="si === sections.length - 1" @click="moveSectionDown(si)" title="Move down">
                    <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
                  </button>
                </div>

                <!-- Section label -->
                <input
                  v-model="section.label"
                  type="text"
                  placeholder="Section label (optional)"
                  class="flex-1 min-w-0 px-2 py-1 text-[12px] font-semibold text-text-muted bg-transparent border-0 focus:outline-none focus:ring-0 placeholder-text-light"
                />

                <!-- Column picker -->
                <div class="flex items-center gap-0.5 shrink-0 border border-border rounded-md bg-white">
                  <button
                    class="px-1.5 py-1 text-[11px] font-medium text-text-muted hover:text-text disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    :disabled="section.columns <= 1"
                    @click="setColumnCount(si, section.columns - 1)"
                    title="Remove column"
                  >&minus;</button>
                  <span class="px-1.5 py-1 text-[11px] font-semibold text-text min-w-[40px] text-center">{{ section.columns }} col</span>
                  <button
                    class="px-1.5 py-1 text-[11px] font-medium text-text-muted hover:text-text transition-colors"
                    @click="setColumnCount(si, section.columns + 1)"
                    title="Add column"
                  >+</button>
                </div>

                <!-- Collapsible toggle -->
                <label class="inline-flex items-center gap-1 text-[11px] text-text-muted cursor-pointer shrink-0" title="Collapsible section">
                  <input v-model="section.collapsible" type="checkbox" class="w-3 h-3 rounded" />
                  Collapse
                </label>

                <!-- Delete section -->
                <button
                  v-if="sections.length > 1"
                  class="p-1 text-text-light hover:text-danger rounded transition-colors shrink-0"
                  @click="removeSection(si)"
                  title="Remove section"
                >
                  <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
                </button>
              </div>

              <!-- Columns grid -->
              <div
                class="p-4"
                :style="{ display: 'grid', gridTemplateColumns: `repeat(${section.columns}, 1fr)`, gap: '1rem' }"
              >
                <div v-for="ci in section.columns" :key="ci" class="space-y-1">
                  <!-- Column header -->
                  <div class="flex items-center justify-between mb-1">
                    <span v-if="section.columns > 1" class="text-[10px] font-semibold text-text-light uppercase tracking-wider">Col {{ ci }}</span>
                    <span v-else />
                    <button
                      class="text-[11px] text-primary-600 hover:text-primary-700 font-medium"
                      @click="addFieldToColumn(si, ci - 1)"
                    >+ Field</button>
                  </div>

                  <!-- Drop zone for entire column -->
                  <div
                    class="space-y-1 min-h-[48px] rounded-lg transition-colors"
                    :class="[
                      dragSource && !(dragSource.si === si && dragSource.ci === ci - 1)
                        ? 'bg-primary-50/30'
                        : '',
                    ]"
                    @dragover="onDragOverEmpty($event, si, ci - 1)"
                    @drop="onDrop($event, si, ci - 1, (section.fields[ci - 1] || []).length)"
                  >
                    <!-- Empty state -->
                    <div
                      v-if="(section.fields[ci - 1] || []).length === 0 && !isDropIndicator(si, ci - 1, 0)"
                      class="border border-dashed border-border rounded-lg px-3 py-6 text-center text-[11px] text-text-light"
                    >
                      {{ dragSource ? 'Drop field here' : 'No fields — click "+ Field" or drag here' }}
                    </div>

                    <!-- Drop indicator at top -->
                    <div
                      v-if="isDropIndicator(si, ci - 1, 0) && (section.fields[ci - 1] || []).length === 0"
                      class="h-1 bg-primary-500 rounded-full mx-2"
                    />

                    <template v-for="(field, fi) in section.fields[ci - 1] || []" :key="fi">
                      <!-- Drop indicator before this field -->
                      <div
                        v-if="isDropIndicator(si, ci - 1, fi)"
                        class="h-1 bg-primary-500 rounded-full mx-2"
                      />

                      <!-- Field card (draggable) -->
                      <div
                        :class="[
                          'flex items-center gap-2 px-3 py-2 rounded-lg border transition-all cursor-grab active:cursor-grabbing group',
                          editingField?.sectionIdx === si && editingField?.colIdx === ci - 1 && editingField?.fieldIdx === fi
                            ? 'border-primary-400 bg-primary-50/50 ring-1 ring-primary-400/30'
                            : 'border-border hover:border-primary-300 bg-white',
                        ]"
                        draggable="true"
                        @dragstart="onDragStart($event, si, ci - 1, fi)"
                        @dragend="onDragEnd"
                        @dragover="onDragOverField($event, si, ci - 1, fi)"
                        @drop.stop="onDrop($event, si, ci - 1, fi)"
                        @click="selectField(si, ci - 1, fi)"
                      >
                        <!-- Drag handle -->
                        <div class="shrink-0 text-text-light/40 group-hover:text-text-light transition-colors cursor-grab">
                          <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
                            <circle cx="9" cy="5" r="1.5" /><circle cx="15" cy="5" r="1.5" />
                            <circle cx="9" cy="12" r="1.5" /><circle cx="15" cy="12" r="1.5" />
                            <circle cx="9" cy="19" r="1.5" /><circle cx="15" cy="19" r="1.5" />
                          </svg>
                        </div>

                        <!-- Field summary -->
                        <div class="flex-1 min-w-0">
                          <div class="flex items-center gap-1.5">
                            <span class="text-[12px] font-medium text-text truncate">
                              {{ field.label || field.fieldname || 'Untitled' }}
                            </span>
                            <span v-if="field.reqd" class="text-danger text-[10px]">*</span>
                          </div>
                          <div class="flex items-center gap-1.5 mt-0.5">
                            <span class="text-[10px] font-mono text-text-light">{{ field.fieldname || '—' }}</span>
                            <span class="text-[10px] text-text-light">{{ field.fieldtype }}</span>
                          </div>
                        </div>

                        <!-- Delete -->
                        <button
                          class="p-0.5 text-text-light hover:text-danger rounded opacity-0 group-hover:opacity-100 transition-all shrink-0"
                          @click.stop="removeField(si, ci - 1, fi)"
                          title="Remove"
                        >
                          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
                        </button>
                      </div>
                    </template>

                    <!-- Drop indicator at bottom -->
                    <div
                      v-if="isDropIndicator(si, ci - 1, (section.fields[ci - 1] || []).length) && (section.fields[ci - 1] || []).length > 0"
                      class="h-1 bg-primary-500 rounded-full mx-2"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Permissions (hidden for child tables — they inherit from parent) -->
          <div v-if="!isChildTable" v-show="activeTab === 'permissions'" class="bg-surface border border-border rounded-xl overflow-hidden">
            <div class="px-5 py-3 border-b border-border bg-surface-muted/30 flex items-center justify-between">
              <h3 class="text-[13px] font-semibold text-text-muted">
                Permissions
                <span v-if="permissions.length" class="text-text-light font-normal">({{ permissions.length }})</span>
              </h3>
              <LButton variant="secondary" size="sm" @click="addPermission">
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
                Add Rule
              </LButton>
            </div>
            <div v-if="permissions.length > 0" class="grid grid-cols-12 gap-2 px-5 py-2 border-b border-border bg-surface-muted/20 text-[11px] font-medium text-text-muted uppercase tracking-wider">
              <div class="col-span-2">Role</div>
              <div class="col-span-1 text-center">Level</div>
              <div class="col-span-1 text-center">Read</div>
              <div class="col-span-1 text-center">Write</div>
              <div class="col-span-1 text-center">Create</div>
              <div class="col-span-1 text-center">Delete</div>
              <div v-if="isSubmittable" class="col-span-1 text-center">Submit</div>
              <div v-if="isSubmittable" class="col-span-1 text-center">Cancel</div>
              <div class="col-span-1"></div>
            </div>
            <div v-if="permissions.length === 0" class="p-8 text-center">
              <p class="text-sm text-text-muted">No permission rules</p>
            </div>
            <div v-else class="divide-y divide-border">
              <div v-for="(perm, pi) in permissions" :key="pi" class="grid grid-cols-12 gap-2 px-5 py-3 items-center group hover:bg-surface-muted/20 transition-colors">
                <div class="col-span-2">
                  <select v-model="perm.role" class="w-full px-2.5 py-[5px] text-sm border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors">
                    <option v-for="role in availableRoles" :key="role" :value="role">{{ role }}</option>
                  </select>
                </div>
                <div class="col-span-1 flex justify-center"><input v-model.number="perm.permlevel" type="number" min="0" max="9" class="w-12 px-1.5 py-[5px] text-sm text-center border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" /></div>
                <div class="col-span-1 flex justify-center"><input v-model="perm.read" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></div>
                <div class="col-span-1 flex justify-center"><input v-model="perm.write" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></div>
                <div class="col-span-1 flex justify-center"><input v-model="perm.create" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></div>
                <div class="col-span-1 flex justify-center"><input v-model="perm.delete" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></div>
                <div v-if="isSubmittable" class="col-span-1 flex justify-center"><input v-model="perm.submit" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></div>
                <div v-if="isSubmittable" class="col-span-1 flex justify-center"><input v-model="perm.cancel" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></div>
                <div class="col-span-1 flex justify-end">
                  <button class="p-1 text-text-light hover:text-danger rounded opacity-30 group-hover:opacity-100 transition-all" @click="removePermission(pi)">
                    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Right panel: field properties -->
        <div
          v-if="editingFieldData"
          class="w-[320px] shrink-0 border-l border-border bg-white overflow-y-auto"
        >
          <div class="px-4 py-3 border-b border-border bg-surface-muted/30 flex items-center justify-between">
            <h3 class="text-[13px] font-semibold text-text-muted">Field Properties</h3>
            <button class="p-1 text-text-light hover:text-text rounded" @click="editingField = null">
              <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
            </button>
          </div>
          <div class="p-4 space-y-3">
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Label</label>
              <input v-model="editingFieldData.label" type="text" placeholder="Field Label" class="w-full px-2.5 py-[5px] text-sm border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" @blur="autoFieldname(editingFieldData)" />
            </div>
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Fieldname</label>
              <input v-model="editingFieldData.fieldname" type="text" placeholder="field_name" class="w-full px-2.5 py-[5px] text-sm font-mono border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" />
            </div>
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Type</label>
              <select v-model="editingFieldData.fieldtype" class="w-full px-2.5 py-[5px] text-sm border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors">
                <option v-for="ft in dataFieldTypes" :key="ft.value" :value="ft.value">{{ ft.value }}</option>
              </select>
            </div>
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Options</label>
              <input v-model="editingFieldData.options" type="text" placeholder="Link target / Select options" class="w-full px-2.5 py-[5px] text-sm border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" />
              <p class="text-[10px] text-text-light mt-0.5">Link: DocType name. Select: newline-separated values.</p>
            </div>
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Fetch From</label>
              <input v-model="editingFieldData.fetch_from" type="text" placeholder="e.g. employee.employee_name" class="w-full px-2.5 py-[5px] text-sm font-mono border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" />
            </div>
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Depends On</label>
              <input v-model="editingFieldData.depends_on" type="text" placeholder="e.g. eval:doc.status=='Active'" class="w-full px-2.5 py-[5px] text-sm font-mono border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" />
            </div>
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Description</label>
              <input v-model="editingFieldData.description" type="text" placeholder="Help text shown below field" class="w-full px-2.5 py-[5px] text-sm border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" />
            </div>
            <div>
              <label class="block text-[11px] font-medium text-text-muted mb-1">Permission Level</label>
              <input v-model.number="editingFieldData.permlevel" type="number" min="0" max="9" class="w-20 px-2.5 py-[5px] text-sm border border-border rounded-md bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors" />
            </div>
            <div class="border-t border-border pt-3 space-y-2">
              <label class="flex items-center gap-2 text-[12px] text-text cursor-pointer">
                <input v-model="editingFieldData.reqd" type="checkbox" class="w-3.5 h-3.5 rounded border-border-strong text-primary-600" />
                Required
              </label>
              <label class="flex items-center gap-2 text-[12px] text-text cursor-pointer">
                <input v-model="editingFieldData.in_list_view" type="checkbox" class="w-3.5 h-3.5 rounded border-border-strong text-primary-600" />
                Show in List View
              </label>
              <label class="flex items-center gap-2 text-[12px] text-text cursor-pointer">
                <input v-model="editingFieldData.in_standard_filter" type="checkbox" class="w-3.5 h-3.5 rounded border-border-strong text-primary-600" />
                Standard Filter
              </label>
              <label class="flex items-center gap-2 text-[12px] text-text cursor-pointer">
                <input v-model="editingFieldData.unique" type="checkbox" class="w-3.5 h-3.5 rounded border-border-strong text-primary-600" />
                Unique
              </label>
              <label class="flex items-center gap-2 text-[12px] text-text cursor-pointer">
                <input v-model="editingFieldData.read_only" type="checkbox" class="w-3.5 h-3.5 rounded border-border-strong text-primary-600" />
                Read Only
              </label>
              <label class="flex items-center gap-2 text-[12px] text-text cursor-pointer">
                <input v-model="editingFieldData.hidden" type="checkbox" class="w-3.5 h-3.5 rounded border-border-strong text-primary-600" />
                Hidden
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
