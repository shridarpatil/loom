<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { loom, type DocTypeMeta, type DocFieldMeta } from "@/utils/call";

const props = defineProps<{
  field: DocFieldMeta;
  modelValue: unknown;
  readOnly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: unknown];
}>();

const childMeta = ref<DocTypeMeta | null>(null);
const loadingMeta = ref(false);
const metaError = ref("");

const rows = computed<Record<string, unknown>[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue;
  if (typeof props.modelValue === "string" && props.modelValue) {
    try {
      const parsed = JSON.parse(props.modelValue);
      if (Array.isArray(parsed)) return parsed;
    } catch { /* ignore */ }
  }
  return [];
});

const visibleColumns = computed<DocFieldMeta[]>(() => {
  if (!childMeta.value) return [];
  const dataFields = childMeta.value.fields.filter(
    (f) => f.fieldtype !== "SectionBreak" && f.fieldtype !== "ColumnBreak" && f.fieldtype !== "TabBreak" && f.fieldtype !== "Table",
  );
  const listViewFields = dataFields.filter((f) => f.in_list_view);
  return listViewFields.length > 0 ? listViewFields : dataFields.slice(0, 5);
});

// Nested Table fields in the child DocType
const nestedTableFields = computed<DocFieldMeta[]>(() => {
  if (!childMeta.value) return [];
  return childMeta.value.fields.filter((f) => f.fieldtype === "Table");
});

onMounted(async () => {
  if (!props.field.options) return;
  loadingMeta.value = true;
  try {
    const res = await loom.getMeta(props.field.options);
    childMeta.value = res.data;
  } catch (e) {
    metaError.value = e instanceof Error ? e.message : "Failed to load child meta";
  } finally {
    loadingMeta.value = false;
  }
});

function emitRows(newRows: Record<string, unknown>[]) {
  emit("update:modelValue", newRows);
}

function addRow() {
  const newRow: Record<string, unknown> = { idx: rows.value.length + 1 };
  if (childMeta.value) {
    for (const f of childMeta.value.fields) {
      if (f.default != null) newRow[f.fieldname] = f.default;
    }
  }
  emitRows([...rows.value, newRow]);
}

function removeRow(idx: number) {
  emitRows(rows.value.filter((_, i) => i !== idx).map((row, i) => ({ ...row, idx: i + 1 })));
}

function updateCell(rowIdx: number, fieldname: string, value: unknown) {
  emitRows(rows.value.map((row, i) => i === rowIdx ? { ...row, [fieldname]: value } : row));
}

function updateNestedTable(rowIdx: number, fieldname: string, value: unknown) {
  emitRows(rows.value.map((row, i) => i === rowIdx ? { ...row, [fieldname]: value } : row));
}

function parseCellValue(event: Event, field: DocFieldMeta): unknown {
  const target = event.target as HTMLInputElement;
  if (field.fieldtype === "Int") return target.value ? parseInt(target.value) : null;
  if (field.fieldtype === "Float" || field.fieldtype === "Currency" || field.fieldtype === "Percent") return target.value ? parseFloat(target.value) : null;
  if (field.fieldtype === "Check") return target.checked ? 1 : 0;
  return target.value;
}

function cellInputType(field: DocFieldMeta): string {
  switch (field.fieldtype) {
    case "Int": case "Float": case "Currency": case "Percent": return "number";
    case "Date": return "date";
    case "Datetime": return "datetime-local";
    case "Time": return "time";
    default: return "text";
  }
}

const cellClass = "w-full h-7 px-1.5 text-[12px] border border-border rounded bg-white focus:outline-none focus:ring-1 focus:ring-primary-500/30 disabled:bg-surface-raised disabled:cursor-not-allowed";
</script>

<template>
  <div>
    <label class="block text-[12px] font-medium text-text-muted mb-1">
      {{ field.label || field.fieldname }}
      <span v-if="field.reqd" class="text-danger">*</span>
    </label>

    <div v-if="loadingMeta" class="text-[12px] text-text-muted py-1">Loading...</div>
    <div v-else-if="metaError" class="text-[12px] text-red-600 py-1">{{ metaError }}</div>

    <div v-else-if="childMeta" class="border border-border rounded-lg overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="bg-surface-muted/50 border-b border-border">
              <th class="px-1.5 py-1.5 text-left text-[10px] font-semibold text-text-light uppercase tracking-wider w-8">#</th>
              <th
                v-for="col in visibleColumns"
                :key="col.fieldname"
                class="px-1.5 py-1.5 text-left text-[10px] font-semibold text-text-light uppercase tracking-wider"
              >{{ col.label || col.fieldname }}</th>
              <th v-if="!readOnly" class="px-1 py-1.5 w-7" />
            </tr>
          </thead>
          <tbody>
            <template v-for="(row, ri) in rows" :key="ri">
              <tr class="border-b border-border" :class="{ 'last:border-b-0': nestedTableFields.length === 0 }">
                <td class="px-1.5 py-1 text-text-light text-[11px]">{{ ri + 1 }}</td>
                <td v-for="col in visibleColumns" :key="col.fieldname" class="px-0.5 py-0.5">
                  <select
                    v-if="col.fieldtype === 'Select'"
                    :value="(row[col.fieldname] as string) ?? ''"
                    :disabled="readOnly"
                    :class="cellClass"
                    @change="updateCell(ri, col.fieldname, ($event.target as HTMLSelectElement).value)"
                  >
                    <option value="">-</option>
                    <option v-for="opt in (col.options || '').split('\n').filter((o: string) => o.trim())" :key="opt" :value="opt">{{ opt }}</option>
                  </select>
                  <input
                    v-else-if="col.fieldtype === 'Check'"
                    type="checkbox"
                    :checked="!!row[col.fieldname]"
                    :disabled="readOnly"
                    class="w-3.5 h-3.5 ml-1"
                    @change="updateCell(ri, col.fieldname, ($event.target as HTMLInputElement).checked ? 1 : 0)"
                  />
                  <input
                    v-else
                    :type="cellInputType(col)"
                    :value="(row[col.fieldname] as string) ?? ''"
                    :disabled="readOnly"
                    :step="col.fieldtype === 'Float' || col.fieldtype === 'Currency' ? 'any' : undefined"
                    :class="cellClass"
                    @input="updateCell(ri, col.fieldname, parseCellValue($event, col))"
                  />
                </td>
                <td v-if="!readOnly" class="px-0.5 py-0.5">
                  <button class="p-0.5 text-text-light hover:text-danger rounded transition-colors" title="Remove" @click="removeRow(ri)">
                    <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
                    </svg>
                  </button>
                </td>
              </tr>
              <!-- Nested Table fields rendered below their parent row -->
              <tr v-for="ntf in nestedTableFields" :key="ntf.fieldname + '-' + ri" class="border-b border-border last:border-b-0">
                <td :colspan="visibleColumns.length + (readOnly ? 1 : 2)" class="px-3 py-1.5 bg-surface-muted/20">
                  <TableControl
                    :field="ntf"
                    :model-value="row[ntf.fieldname]"
                    :read-only="readOnly"
                    @update:model-value="updateNestedTable(ri, ntf.fieldname, $event)"
                  />
                </td>
              </tr>
            </template>
            <tr v-if="rows.length === 0">
              <td :colspan="visibleColumns.length + (readOnly ? 1 : 2)" class="px-2 py-3 text-center text-[12px] text-text-light">
                No rows.{{ readOnly ? '' : ' Click "Add Row" below.' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-if="!readOnly" class="px-2 py-1.5 border-t border-border">
        <button class="text-[12px] text-primary-600 hover:text-primary-700 font-medium" @click="addRow">+ Add Row</button>
      </div>
    </div>

    <div v-else class="text-[12px] text-text-muted py-1">No child DocType specified.</div>
  </div>
</template>
