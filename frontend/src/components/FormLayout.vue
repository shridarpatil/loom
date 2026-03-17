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

// Track collapsed state per section index
const collapsedSections = ref<Set<number>>(new Set());

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
  // Check permlevel — hide if user cannot read this level
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
  // Check permlevel — read-only if user can read but not write this level
  const level = field.permlevel || 0;
  if (!allowedWriteLevels.value.has(level)) return true;
  if (field.read_only_depends_on) {
    return evaluateDependsOn(field.read_only_depends_on, props.modelValue);
  }
  return false;
}

const sections = computed<Section[]>(() => {
  const result: Section[] = [];
  let currentSection: Section = {
    label: "",
    collapsible: false,
    columns: [{ fields: [] }],
  };
  result.push(currentSection);

  for (const field of props.fields) {
    if (field.fieldtype === "SectionBreak" || field.fieldtype === "TabBreak") {
      currentSection = {
        label: field.label || "",
        collapsible: field.collapsible || false,
        columns: [{ fields: [] }],
      };
      result.push(currentSection);
    } else if (field.fieldtype === "ColumnBreak") {
      currentSection.columns.push({ fields: [] });
    } else {
      const lastCol = currentSection.columns[currentSection.columns.length - 1];
      lastCol.fields.push(field);
    }
  }

  return result.filter((s) =>
    s.columns.some((c) => c.fields.length > 0),
  );
});

// Compute max width based on the maximum number of columns across all sections
const maxColumns = computed(() => {
  return Math.max(1, ...sections.value.map((s) => s.columns.length));
});

const formMaxWidth = computed(() => {
  // ~320px per column
  const widths: Record<number, string> = {
    1: "640px",
    2: "960px",
    3: "1200px",
    4: "1440px",
  };
  return widths[maxColumns.value] || `${maxColumns.value * 320}px`;
});

function toggleCollapse(si: number) {
  const s = new Set(collapsedSections.value);
  if (s.has(si)) {
    s.delete(si);
  } else {
    s.add(si);
  }
  collapsedSections.value = s;
}

// Auto-collapse collapsible sections when sections change
watch(sections, (secs) => {
  const toCollapse = new Set<number>();
  secs.forEach((s, i) => {
    if (s.collapsible) toCollapse.add(i);
  });
  collapsedSections.value = toCollapse;
}, { immediate: true });

function updateField(fieldname: string, value: unknown) {
  emit("update:modelValue", {
    ...props.modelValue,
    [fieldname]: value,
  });
}
</script>

<template>
  <div class="space-y-5" :style="{ maxWidth: formMaxWidth, margin: '0 auto' }">
    <div
      v-for="(section, si) in sections"
      :key="si"
      class="bg-white border border-border/60 rounded-xl shadow-sm shadow-black/[0.02] overflow-hidden"
    >
      <!-- Section header -->
      <div
        v-if="section.label"
        :class="[
          'px-5 py-3 flex items-center justify-between',
          collapsedSections.has(si) ? '' : 'border-b border-border/60',
          section.collapsible ? 'cursor-pointer select-none hover:bg-surface-muted/40 transition-colors' : '',
        ]"
        @click="section.collapsible ? toggleCollapse(si) : undefined"
      >
        <h3 class="text-[13px] font-semibold text-text">{{ section.label }}</h3>
        <svg
          v-if="section.collapsible"
          :class="[
            'w-4 h-4 text-text-light transition-transform duration-200',
            collapsedSections.has(si) ? '-rotate-90' : '',
          ]"
          fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
        >
          <path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
        </svg>
      </div>

      <!-- Columns grid (hidden when collapsed) -->
      <div
        v-show="!collapsedSections.has(si)"
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
</template>
