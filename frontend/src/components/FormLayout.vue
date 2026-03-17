<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { DocFieldMeta, DocPermMeta } from "@/utils/call";
import { evaluateDependsOn } from "@/utils/depends";
import FieldControl from "./controls/FieldControl.vue";

const props = defineProps<{
  fields: DocFieldMeta[];
  modelValue: Record<string, unknown>;
  readOnly?: boolean;
  permissions?: DocPermMeta[];
  userRoles?: string[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: Record<string, unknown>];
}>();

interface Section {
  label: string;
  collapsible: boolean;
  columns: Column[];
}

interface Column {
  fields: DocFieldMeta[];
}

interface Tab {
  label: string;
  sections: Section[];
}

// Track collapsed state per section
const collapsedSections = ref<Set<string>>(new Set());
const activeTabIdx = ref(0);

// Compute which permlevels the user can read and write
const allowedReadLevels = computed<Set<number>>(() => {
  const levels = new Set<number>();
  if (!props.permissions || !props.userRoles) {
    for (let i = 0; i <= 9; i++) levels.add(i);
    return levels;
  }
  for (const perm of props.permissions) {
    if (perm.read && props.userRoles.includes(perm.role)) {
      levels.add(perm.permlevel);
    }
  }
  // Higher level read implies level 0 visibility
  if (levels.size > 0) levels.add(0);
  return levels;
});

const allowedWriteLevels = computed<Set<number>>(() => {
  const levels = new Set<number>();
  if (!props.permissions || !props.userRoles) {
    for (let i = 0; i <= 9; i++) levels.add(i);
    return levels;
  }
  for (const perm of props.permissions) {
    if (perm.write && props.userRoles.includes(perm.role)) {
      levels.add(perm.permlevel);
    }
  }
  return levels;
});

function isFieldVisible(field: DocFieldMeta): boolean {
  if (field.hidden) return false;
  const level = field.permlevel || 0;
  if (!allowedReadLevels.value.has(level)) return false;
  if (field.depends_on) {
    return evaluateDependsOn(field.depends_on, props.modelValue);
  }
  return true;
}

function isFieldRequired(field: DocFieldMeta): boolean {
  if (field.mandatory_depends_on) {
    return evaluateDependsOn(field.mandatory_depends_on, props.modelValue);
  }
  return field.reqd || false;
}

function isFieldReadOnly(field: DocFieldMeta): boolean {
  if (props.readOnly) return true;
  if (field.read_only) return true;
  const level = field.permlevel || 0;
  if (!allowedWriteLevels.value.has(level)) return true;
  if (field.read_only_depends_on) {
    return evaluateDependsOn(field.read_only_depends_on, props.modelValue);
  }
  return false;
}

// Parse fields into tabs → sections → columns → fields
const tabs = computed<Tab[]>(() => {
  const result: Tab[] = [];
  let currentTab: Tab = { label: "", sections: [] };
  let currentSection: Section = { label: "", collapsible: false, columns: [{ fields: [] }] };
  currentTab.sections.push(currentSection);
  result.push(currentTab);

  for (const field of props.fields) {
    if (field.fieldtype === "TabBreak") {
      // Start a new tab
      currentTab = { label: field.label || "", sections: [] };
      currentSection = { label: "", collapsible: false, columns: [{ fields: [] }] };
      currentTab.sections.push(currentSection);
      result.push(currentTab);
    } else if (field.fieldtype === "SectionBreak") {
      currentSection = {
        label: field.label || "",
        collapsible: field.collapsible || false,
        columns: [{ fields: [] }],
      };
      currentTab.sections.push(currentSection);
    } else if (field.fieldtype === "ColumnBreak") {
      currentSection.columns.push({ fields: [] });
    } else {
      const lastCol = currentSection.columns[currentSection.columns.length - 1];
      lastCol.fields.push(field);
    }
  }

  // Filter out empty sections and tabs
  return result
    .map((tab) => ({
      ...tab,
      sections: tab.sections.filter((s) => s.columns.some((c) => c.fields.length > 0)),
    }))
    .filter((tab) => tab.sections.length > 0);
});

const hasTabs = computed(() => tabs.value.length > 1);

// Compute max width based on the maximum number of columns
const maxColumns = computed(() => {
  let max = 1;
  for (const tab of tabs.value) {
    for (const sec of tab.sections) {
      max = Math.max(max, sec.columns.length);
    }
  }
  return max;
});

const formMaxWidth = computed(() => {
  const widths: Record<number, string> = { 1: "640px", 2: "960px", 3: "1200px", 4: "1440px" };
  return widths[maxColumns.value] || `${maxColumns.value * 320}px`;
});

function sectionKey(tabIdx: number, secIdx: number) {
  return `${tabIdx}-${secIdx}`;
}

function toggleCollapse(tabIdx: number, secIdx: number) {
  const key = sectionKey(tabIdx, secIdx);
  const s = new Set(collapsedSections.value);
  if (s.has(key)) { s.delete(key); } else { s.add(key); }
  collapsedSections.value = s;
}

function isCollapsed(tabIdx: number, secIdx: number) {
  return collapsedSections.value.has(sectionKey(tabIdx, secIdx));
}

// Auto-collapse collapsible sections
watch(tabs, (t) => {
  const toCollapse = new Set<string>();
  t.forEach((tab, ti) => {
    tab.sections.forEach((s, si) => {
      if (s.collapsible) toCollapse.add(sectionKey(ti, si));
    });
  });
  collapsedSections.value = toCollapse;
}, { immediate: true });

// Reset active tab when fields change
watch(tabs, () => {
  if (activeTabIdx.value >= tabs.value.length) {
    activeTabIdx.value = 0;
  }
});

function updateField(fieldname: string, value: unknown) {
  emit("update:modelValue", {
    ...props.modelValue,
    [fieldname]: value,
  });
}
</script>

<template>
  <div :style="{ maxWidth: formMaxWidth, margin: '0 auto' }">
    <!-- Tab bar (only if multiple tabs) -->
    <div v-if="hasTabs" class="border-b border-border bg-white rounded-t-xl mb-0 px-1">
      <div class="flex gap-0 overflow-x-auto">
        <button
          v-for="(tab, ti) in tabs"
          :key="ti"
          :class="[
            'px-4 py-2.5 text-[13px] font-medium border-b-2 transition-colors -mb-px whitespace-nowrap',
            activeTabIdx === ti
              ? 'border-primary-600 text-primary-700'
              : 'border-transparent text-text-muted hover:text-text hover:border-border',
          ]"
          @click="activeTabIdx = ti"
        >{{ tab.label || `Tab ${ti + 1}` }}</button>
      </div>
    </div>

    <!-- Tab content -->
    <div v-for="(tab, ti) in tabs" :key="ti" v-show="!hasTabs || activeTabIdx === ti" class="space-y-5" :class="{ 'pt-5': hasTabs }">
      <div
        v-for="(section, si) in tab.sections"
        :key="si"
        class="bg-white border border-border/60 rounded-xl shadow-sm shadow-black/[0.02] overflow-hidden"
      >
        <!-- Section header -->
        <div
          v-if="section.label"
          :class="[
            'px-5 py-3 flex items-center justify-between',
            isCollapsed(ti, si) ? '' : 'border-b border-border/60',
            section.collapsible ? 'cursor-pointer select-none hover:bg-surface-muted/40 transition-colors' : '',
          ]"
          @click="section.collapsible ? toggleCollapse(ti, si) : undefined"
        >
          <h3 class="text-[13px] font-semibold text-text">{{ section.label }}</h3>
          <svg
            v-if="section.collapsible"
            :class="[
              'w-4 h-4 text-text-light transition-transform duration-200',
              isCollapsed(ti, si) ? '-rotate-90' : '',
            ]"
            fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
          >
            <path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
          </svg>
        </div>

        <!-- Columns grid -->
        <div
          v-show="!isCollapsed(ti, si)"
          class="px-5 py-4"
          :style="
            section.columns.length > 1
              ? { display: 'grid', gridTemplateColumns: `repeat(${section.columns.length}, 1fr)`, gap: '1.25rem' }
              : {}
          "
        >
          <div v-for="(col, ci) in section.columns" :key="ci" class="space-y-4">
            <template v-for="field in col.fields" :key="field.fieldname">
              <FieldControl
                v-if="isFieldVisible(field)"
                :field="field"
                :modelValue="modelValue[field.fieldname]"
                :readOnly="isFieldReadOnly(field)"
                :computedReqd="isFieldRequired(field)"
                @update:modelValue="updateField(field.fieldname, $event)"
              />
            </template>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
