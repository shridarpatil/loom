<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { loom, type DocTypeMeta, type DocFieldMeta } from "@/utils/call";
import { LButton, LAlert, LPageHeader } from "@/components/ui";

interface ReportFilter {
  field: string;
  operator: string;
  value: string;
  value2: string;
}

const doctypeList = ref<string[]>([]);
const loadingDoctypes = ref(true);
const selectedDoctype = ref("");
const meta = ref<DocTypeMeta | null>(null);
const loadingMeta = ref(false);
const selectedFields = ref<Set<string>>(new Set());
const filters = ref<ReportFilter[]>([]);
const results = ref<Record<string, unknown>[]>([]);
const loadingResults = ref(false);
const resultError = ref("");
const orderBy = ref("");
const limit = ref(100);

const numericTypes = ["Int", "Float", "Currency", "Percent"];
const dateTypes = ["Date", "Datetime"];
const textTypes = ["Data", "Text", "SmallText", "LongText", "Link", "Select", "Code"];

const dataFields = computed<DocFieldMeta[]>(() => {
  if (!meta.value) return [];
  return meta.value.fields.filter(
    (f) =>
      f.fieldtype !== "SectionBreak" &&
      f.fieldtype !== "ColumnBreak" &&
      f.fieldtype !== "TabBreak" &&
      f.fieldtype !== "Table",
  );
});

const resultColumns = computed<string[]>(() => {
  const cols = Array.from(selectedFields.value);
  if (!cols.includes("id")) cols.unshift("id");
  return cols;
});

function operatorsForField(fieldname: string): { value: string; label: string }[] {
  const field = dataFields.value.find((f) => f.fieldname === fieldname);
  const ft = field?.fieldtype || "Data";
  const base = [
    { value: "=", label: "=" },
    { value: "!=", label: "!=" },
  ];
  if (numericTypes.includes(ft) || dateTypes.includes(ft) || ft === "Time") {
    return [...base, { value: ">", label: ">" }, { value: "<", label: "<" }, { value: ">=", label: ">=" }, { value: "<=", label: "<=" }, { value: "between", label: "between" }];
  }
  if (textTypes.includes(ft)) {
    return [...base, { value: "like", label: "like" }, { value: "not like", label: "not like" }];
  }
  return base;
}

function inputTypeForField(fieldname: string): string {
  const field = dataFields.value.find((f) => f.fieldname === fieldname);
  if (!field) return "text";
  switch (field.fieldtype) {
    case "Int": case "Float": case "Currency": case "Percent": return "number";
    case "Date": return "date";
    case "Datetime": return "datetime-local";
    case "Time": return "time";
    default: return "text";
  }
}

function inputStep(fieldname: string): string | undefined {
  const field = dataFields.value.find((f) => f.fieldname === fieldname);
  if (!field) return undefined;
  if (field.fieldtype === "Float" || field.fieldtype === "Currency" || field.fieldtype === "Percent") return "any";
  return undefined;
}

function isSelectField(fieldname: string): boolean {
  return dataFields.value.find((f) => f.fieldname === fieldname)?.fieldtype === "Select";
}

function selectOptions(fieldname: string): string[] {
  const field = dataFields.value.find((f) => f.fieldname === fieldname);
  if (!field?.options) return [];
  return field.options.split("\n").filter((o) => o.trim());
}

function isCheckField(fieldname: string): boolean {
  return dataFields.value.find((f) => f.fieldname === fieldname)?.fieldtype === "Check";
}

async function loadDoctypes() {
  loadingDoctypes.value = true;
  try {
    const res = await loom.resource("DocType").getList({ limit: 200 });
    doctypeList.value = res.data.map((d) => (d.name || d.id) as string).sort();
  } catch {
    doctypeList.value = [];
  } finally {
    loadingDoctypes.value = false;
  }
}
loadDoctypes();

watch(selectedDoctype, async (dt) => {
  meta.value = null;
  selectedFields.value = new Set();
  filters.value = [];
  results.value = [];
  resultError.value = "";
  if (!dt) return;
  loadingMeta.value = true;
  try {
    const res = await loom.getMeta(dt);
    meta.value = res.data;
    const preselect = new Set<string>(["id"]);
    for (const f of res.data.fields) { if (f.in_list_view) preselect.add(f.fieldname); }
    selectedFields.value = preselect;
  } catch (e) {
    resultError.value = e instanceof Error ? e.message : "Failed to load meta";
  } finally {
    loadingMeta.value = false;
  }
});

function toggleField(fieldname: string) {
  const s = new Set(selectedFields.value);
  if (s.has(fieldname)) s.delete(fieldname); else s.add(fieldname);
  selectedFields.value = s;
}

function addFilter() {
  filters.value = [...filters.value, { field: dataFields.value[0]?.fieldname || "", operator: "=", value: "", value2: "" }];
}

function removeFilter(idx: number) {
  filters.value = filters.value.filter((_, i) => i !== idx);
}

function updateFilter(idx: number, key: keyof ReportFilter, val: string) {
  filters.value = filters.value.map((f, i) => {
    if (i !== idx) return f;
    const updated = { ...f, [key]: val };
    if (key === "field") {
      const validOps = operatorsForField(val).map((o) => o.value);
      if (!validOps.includes(updated.operator)) updated.operator = "=";
      updated.value = "";
      updated.value2 = "";
    }
    return updated;
  });
}

async function runReport() {
  if (!selectedDoctype.value) return;
  loadingResults.value = true;
  resultError.value = "";
  try {
    const fields = Array.from(selectedFields.value);
    if (!fields.includes("id")) fields.unshift("id");
    const filterArr: unknown[] = [];
    for (const f of filters.value) {
      if (!f.field || f.value === "") continue;
      if (f.operator === "between") {
        if (f.value && f.value2) filterArr.push([f.field, "between", [f.value, f.value2]]);
      } else if (f.operator === "like" || f.operator === "not like") {
        let pattern = f.value;
        if (!pattern.includes("%")) pattern = `%${pattern}%`;
        filterArr.push([f.field, f.operator, pattern]);
      } else {
        filterArr.push([f.field, f.operator, f.value]);
      }
    }
    const res = await loom.resource(selectedDoctype.value).getList({
      fields,
      filters: filterArr.length > 0 ? filterArr : undefined,
      order_by: orderBy.value || undefined,
      limit: limit.value,
    });
    results.value = res.data;
  } catch (e) {
    resultError.value = e instanceof Error ? e.message : "Query failed";
    results.value = [];
  } finally {
    loadingResults.value = false;
  }
}

function downloadCSV() {
  if (results.value.length === 0) return;
  const cols = resultColumns.value;
  const header = cols.join(",");
  const rows = results.value.map((row) =>
    cols.map((c) => {
      const val = row[c];
      if (val == null) return "";
      const str = String(val);
      if (str.includes(",") || str.includes('"') || str.includes("\n")) return `"${str.replace(/"/g, '""')}"`;
      return str;
    }).join(","),
  );
  const csv = [header, ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${selectedDoctype.value}_report.csv`;
  a.click();
  URL.revokeObjectURL(url);
}

function sortBy(col: string) {
  orderBy.value = orderBy.value === `${col} asc` ? `${col} desc` : `${col} asc`;
}
</script>

<template>
  <div class="h-full flex flex-col">
    <LPageHeader title="Report Builder" />

    <div class="flex-1 overflow-auto px-6 py-4">
      <div class="space-y-4">
        <!-- DocType selector -->
        <div class="bg-white border border-border rounded-lg px-4 py-3">
          <label class="block text-[12px] font-medium text-text-muted mb-1">DocType</label>
          <select
            v-model="selectedDoctype"
            class="w-full max-w-xs h-8 px-2.5 text-[13px] border border-border rounded-md bg-white text-text focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400"
          >
            <option value="">Select a DocType...</option>
            <option v-for="dt in doctypeList" :key="dt" :value="dt">{{ dt }}</option>
          </select>
        </div>

        <template v-if="meta">
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <!-- Field picker -->
            <div class="bg-white border border-border rounded-lg px-4 py-3">
              <h3 class="text-[12px] font-semibold text-text-muted uppercase tracking-wide mb-2">Columns</h3>
              <div class="space-y-0.5 max-h-52 overflow-y-auto">
                <label class="flex items-center gap-1.5 text-[13px] text-text-muted cursor-not-allowed py-0.5">
                  <input type="checkbox" checked disabled class="w-3 h-3" /> id
                </label>
                <label
                  v-for="field in dataFields"
                  :key="field.fieldname"
                  class="flex items-center gap-1.5 text-[13px] cursor-pointer py-0.5 transition-colors"
                  :class="selectedFields.has(field.fieldname) ? 'text-text font-medium' : 'text-text-muted hover:text-text'"
                >
                  <input type="checkbox" :checked="selectedFields.has(field.fieldname)" class="w-3 h-3 accent-primary-600" @change="toggleField(field.fieldname)" />
                  {{ field.label || field.fieldname }}
                </label>
              </div>
            </div>

            <!-- Filters -->
            <div class="bg-white border border-border rounded-lg px-4 py-3">
              <h3 class="text-[12px] font-semibold text-text-muted uppercase tracking-wide mb-2">Filters</h3>
              <div class="space-y-1.5">
                <div v-for="(filter, fi) in filters" :key="fi" class="flex items-center gap-1">
                  <select :value="filter.field" class="w-[120px] shrink-0 h-7 px-1.5 text-[12px] border border-border rounded-md bg-white" @change="updateFilter(fi, 'field', ($event.target as HTMLSelectElement).value)">
                    <option v-for="f in dataFields" :key="f.fieldname" :value="f.fieldname">{{ f.label || f.fieldname }}</option>
                  </select>
                  <select :value="filter.operator" class="w-[72px] shrink-0 h-7 px-1.5 text-[12px] border border-border rounded-md bg-white" @change="updateFilter(fi, 'operator', ($event.target as HTMLSelectElement).value)">
                    <option v-for="op in operatorsForField(filter.field)" :key="op.value" :value="op.value">{{ op.label }}</option>
                  </select>
                  <template v-if="filter.operator === 'between'">
                    <input :value="filter.value" :type="inputTypeForField(filter.field)" :step="inputStep(filter.field)" class="flex-1 min-w-0 h-7 px-1.5 text-[12px] border border-border rounded-md bg-white" placeholder="From" @input="updateFilter(fi, 'value', ($event.target as HTMLInputElement).value)" />
                    <span class="text-[11px] text-text-light shrink-0">to</span>
                    <input :value="filter.value2" :type="inputTypeForField(filter.field)" :step="inputStep(filter.field)" class="flex-1 min-w-0 h-7 px-1.5 text-[12px] border border-border rounded-md bg-white" placeholder="To" @input="updateFilter(fi, 'value2', ($event.target as HTMLInputElement).value)" />
                  </template>
                  <template v-else-if="isCheckField(filter.field)">
                    <select :value="filter.value" class="flex-1 min-w-0 h-7 px-1.5 text-[12px] border border-border rounded-md bg-white" @change="updateFilter(fi, 'value', ($event.target as HTMLSelectElement).value)">
                      <option value="">-</option><option value="true">Yes</option><option value="false">No</option>
                    </select>
                  </template>
                  <template v-else-if="isSelectField(filter.field) && (filter.operator === '=' || filter.operator === '!=')">
                    <select :value="filter.value" class="flex-1 min-w-0 h-7 px-1.5 text-[12px] border border-border rounded-md bg-white" @change="updateFilter(fi, 'value', ($event.target as HTMLSelectElement).value)">
                      <option value="">-</option>
                      <option v-for="opt in selectOptions(filter.field)" :key="opt" :value="opt">{{ opt }}</option>
                    </select>
                  </template>
                  <template v-else>
                    <input :value="filter.value" :type="inputTypeForField(filter.field)" :step="inputStep(filter.field)" class="flex-1 min-w-0 h-7 px-1.5 text-[12px] border border-border rounded-md bg-white" placeholder="Value" @input="updateFilter(fi, 'value', ($event.target as HTMLInputElement).value)" />
                  </template>
                  <button class="p-0.5 text-text-light hover:text-danger transition-colors shrink-0" @click="removeFilter(fi)">
                    <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
                  </button>
                </div>
                <button class="text-[12px] text-primary-600 hover:text-primary-700 font-medium" @click="addFilter">+ Add Filter</button>
              </div>
            </div>
          </div>

          <!-- Run + Download -->
          <div class="flex items-center gap-2">
            <LButton :disabled="loadingResults" :loading="loadingResults" @click="runReport">Run</LButton>
            <LButton v-if="results.length > 0" variant="secondary" @click="downloadCSV">Download CSV</LButton>
            <span v-if="results.length > 0" class="text-[12px] text-text-muted">{{ results.length }} row{{ results.length === 1 ? '' : 's' }}</span>
          </div>
        </template>

        <!-- Error -->
        <LAlert v-if="resultError" type="error">{{ resultError }}</LAlert>

        <!-- Results table -->
        <div v-if="results.length > 0" class="bg-white border border-border rounded-lg overflow-hidden">
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead>
                <tr class="border-b border-border">
                  <th
                    v-for="col in resultColumns"
                    :key="col"
                    class="px-3 py-2 text-left text-[11px] font-semibold text-text-light uppercase tracking-wider cursor-pointer hover:text-text transition-colors select-none bg-surface-muted/50"
                    @click="sortBy(col)"
                  >
                    {{ col }}
                    <span v-if="orderBy === `${col} asc`" class="ml-0.5">&#9650;</span>
                    <span v-else-if="orderBy === `${col} desc`" class="ml-0.5">&#9660;</span>
                  </th>
                </tr>
              </thead>
              <tbody class="text-[13px]">
                <tr
                  v-for="(row, ri) in results"
                  :key="ri"
                  class="border-b border-border last:border-b-0 hover:bg-surface-muted/40"
                >
                  <td v-for="col in resultColumns" :key="col" class="px-3 py-2 text-text">
                    <router-link v-if="col === 'id'" :to="`/app/${selectedDoctype}/${row[col]}`" class="text-primary-600 hover:underline font-medium">{{ row[col] ?? '' }}</router-link>
                    <template v-else>{{ row[col] ?? '' }}</template>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
