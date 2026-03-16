<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useRouter } from "vue-router";
import { loom, type DocTypeMeta, type DocFieldMeta } from "@/utils/call";
import { useSession } from "@/composables/useSession";
import { LButton, LAlert, LPageHeader } from "@/components/ui";

const props = defineProps<{ doctype: string }>();
const router = useRouter();
const { isAdmin } = useSession();

const meta = ref<DocTypeMeta | null>(null);
const overrides = ref<Record<string, Record<string, unknown>>>({});
const clientScript = ref("");
const serverScript = ref("");
const loading = ref(true);
const saving = ref(false);
const error = ref("");
const success = ref("");

function isDataField(f: DocFieldMeta): boolean {
  return !["SectionBreak", "ColumnBreak", "TabBreak"].includes(f.fieldtype);
}

const dataFields = computed(() => {
  if (!meta.value) return [];
  return meta.value.fields.filter(isDataField);
});

function getOverride(fieldname: string, prop: string, fallback: unknown): unknown {
  return overrides.value[fieldname]?.[prop] ?? fallback;
}

function setOverride(fieldname: string, prop: string, value: unknown, original: unknown) {
  if (!overrides.value[fieldname]) {
    overrides.value[fieldname] = {};
  }
  if (value === original) {
    delete overrides.value[fieldname][prop];
    if (Object.keys(overrides.value[fieldname]).length === 0) {
      delete overrides.value[fieldname];
    }
  } else {
    overrides.value[fieldname][prop] = value;
  }
}

onMounted(async () => {
  try {
    const metaRes = await loom.getMeta(props.doctype);
    meta.value = metaRes.data;

    const res = await fetch(`/api/customize/${encodeURIComponent(props.doctype)}`);
    const data = await res.json();
    if (data.data) {
      overrides.value = data.data.overrides || {};
      clientScript.value = data.data.client_script || "";
      serverScript.value = data.data.server_script || "";
    }
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Failed to load";
  } finally {
    loading.value = false;
  }
});

async function save() {
  saving.value = true;
  error.value = "";
  success.value = "";
  try {
    const res = await fetch(`/api/customize/${encodeURIComponent(props.doctype)}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        overrides: overrides.value,
        client_script: clientScript.value,
        server_script: serverScript.value,
      }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || "Save failed");
    success.value = "Customization saved";
    setTimeout(() => { success.value = ""; }, 3000);
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Save failed";
  } finally {
    saving.value = false;
  }
}

function reset() {
  overrides.value = {};
  clientScript.value = "";
  serverScript.value = "";
}

async function exportCustomization() {
  try {
    const res = await fetch(`/api/customize/${encodeURIComponent(props.doctype)}/export`, {
      method: "POST",
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || "Export failed");
    success.value = data.message;
    setTimeout(() => { success.value = ""; }, 5000);
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Export failed";
  }
}

const overrideCount = computed(() => Object.keys(overrides.value).length);
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <LPageHeader :title="`Customize Form: ${doctype}`" subtitle="Site-level overrides. Does not modify the app's DocType definition.">
      <template #breadcrumb>
        <button
          class="inline-flex items-center gap-1 text-[13px] text-text-muted hover:text-primary-600 transition-colors mb-1"
          @click="router.push(`/app/${doctype}`)"
        >
          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5" />
          </svg>
          {{ doctype }}
        </button>
      </template>
      <template #actions>
        <LButton
          v-if="isAdmin() && (overrideCount > 0 || clientScript || serverScript)"
          variant="secondary"
          @click="exportCustomization"
        >
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
          </svg>
          Export
        </LButton>
        <LButton
          v-if="overrideCount > 0 || clientScript || serverScript"
          variant="secondary"
          @click="reset"
        >Reset</LButton>
        <LButton :disabled="saving" :loading="saving" @click="save">
          Save
        </LButton>
      </template>
    </LPageHeader>

    <!-- Alerts -->
    <div class="px-8 pt-4 space-y-2" v-if="error || success">
      <LAlert v-if="error" type="error" dismissible @dismiss="error = ''">{{ error }}</LAlert>
      <LAlert v-if="success" type="success">{{ success }}</LAlert>
    </div>

    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <span class="text-sm text-text-muted">Loading...</span>
    </div>

    <div v-else-if="meta" class="flex-1 overflow-auto">
      <div class="p-8 space-y-5">

        <!-- Field Overrides -->
        <div class="bg-surface border border-border rounded-xl overflow-hidden">
          <div class="px-5 py-3 border-b border-border bg-surface-muted/30">
            <h3 class="text-[13px] font-semibold text-text-muted">
              Field Properties
              <span v-if="overrideCount > 0" class="text-primary-600 font-normal">({{ overrideCount }} customized)</span>
            </h3>
          </div>

          <div class="grid grid-cols-12 gap-2 px-5 py-2 border-b border-border bg-surface-muted/20 text-[11px] font-medium text-text-muted uppercase tracking-wider">
            <div class="col-span-3">Field</div>
            <div class="col-span-2">Type</div>
            <div class="col-span-1 text-center">Hidden</div>
            <div class="col-span-1 text-center">Required</div>
            <div class="col-span-1 text-center">Read Only</div>
            <div class="col-span-4">Default</div>
          </div>

          <div class="divide-y divide-border">
            <div
              v-for="field in dataFields"
              :key="field.fieldname"
              :class="[
                'grid grid-cols-12 gap-2 px-5 py-2.5 items-center text-sm transition-colors',
                overrides[field.fieldname] ? 'bg-primary-50/30' : 'hover:bg-surface-muted/20'
              ]"
            >
              <div class="col-span-3">
                <span class="font-medium text-text">{{ field.label || field.fieldname }}</span>
                <span class="block text-[11px] font-mono text-text-light">{{ field.fieldname }}</span>
              </div>
              <div class="col-span-2 text-text-muted text-xs">{{ field.fieldtype }}</div>
              <div class="col-span-1 flex justify-center">
                <input
                  type="checkbox"
                  :checked="!!getOverride(field.fieldname, 'hidden', field.hidden)"
                  class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500"
                  @change="setOverride(field.fieldname, 'hidden', ($event.target as HTMLInputElement).checked, field.hidden || false)"
                />
              </div>
              <div class="col-span-1 flex justify-center">
                <input
                  type="checkbox"
                  :checked="!!getOverride(field.fieldname, 'reqd', field.reqd)"
                  class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500"
                  @change="setOverride(field.fieldname, 'reqd', ($event.target as HTMLInputElement).checked, field.reqd || false)"
                />
              </div>
              <div class="col-span-1 flex justify-center">
                <input
                  type="checkbox"
                  :checked="!!getOverride(field.fieldname, 'read_only', field.read_only)"
                  class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500"
                  @change="setOverride(field.fieldname, 'read_only', ($event.target as HTMLInputElement).checked, field.read_only || false)"
                />
              </div>
              <div class="col-span-4">
                <input
                  type="text"
                  :value="getOverride(field.fieldname, 'default', field.default ?? '') as string"
                  placeholder="Default value"
                  class="w-full px-2 py-1 text-xs border border-border rounded bg-surface focus:outline-none focus:ring-1 focus:ring-primary-400 transition-colors"
                  @input="setOverride(field.fieldname, 'default', ($event.target as HTMLInputElement).value || undefined, field.default ?? '')"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Client Script -->
        <div class="bg-surface border border-border rounded-xl overflow-hidden">
          <div class="px-5 py-3 border-b border-border bg-surface-muted/30">
            <h3 class="text-[13px] font-semibold text-text-muted">Client Script</h3>
            <p class="text-[11px] text-text-light mt-0.5">
              JavaScript that runs on the form. Use <code class="bg-surface-raised px-1 rounded">loom.validate(doc)</code> to add validation.
            </p>
          </div>
          <div class="p-4">
            <textarea
              v-model="clientScript"
              rows="12"
              placeholder="// Example: validate due_date is not in the past
loom.validate = function(doc) {
  if (doc.due_date && new Date(doc.due_date) < new Date().setHours(0,0,0,0)) {
    return 'Due Date cannot be in the past';
  }
};

// Example: auto-set a field when another changes
loom.on_change = function(doc, fieldname) {
  if (fieldname === 'status' && doc.status === 'Completed') {
    doc.completed_date = new Date().toISOString().slice(0, 10);
  }
};"
              class="w-full px-4 py-3 text-[13px] font-mono border border-border rounded-lg bg-surface text-text placeholder-text-light/60 focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-colors"
            />
          </div>
        </div>

        <!-- Server Script (Rhai) -->
        <div class="bg-surface border border-border rounded-xl overflow-hidden">
          <div class="px-5 py-3 border-b border-border bg-surface-muted/30">
            <h3 class="text-[13px] font-semibold text-text-muted">Server Script</h3>
            <p class="text-[11px] text-text-light mt-0.5">
              Rhai script that runs on the server during document lifecycle.
              Available hooks: <code class="bg-surface-raised px-1 rounded">validate</code>,
              <code class="bg-surface-raised px-1 rounded">before_save</code>,
              <code class="bg-surface-raised px-1 rounded">before_insert</code>,
              <code class="bg-surface-raised px-1 rounded">on_update</code>,
              <code class="bg-surface-raised px-1 rounded">on_submit</code>,
              <code class="bg-surface-raised px-1 rounded">on_cancel</code>,
              <code class="bg-surface-raised px-1 rounded">on_trash</code>.
              APIs: <code class="bg-surface-raised px-1 rounded">throw(msg)</code>,
              <code class="bg-surface-raised px-1 rounded">today()</code>,
              <code class="bg-surface-raised px-1 rounded">log(msg)</code>.
              Hot-reloaded on save.
            </p>
          </div>
          <div class="p-4">
            <textarea
              v-model="serverScript"
              rows="12"
              placeholder="// Example: server-side validation
fn validate(doc) {
    if doc.title == &quot;&quot; {
        throw(&quot;Title is required&quot;);
    }
}

fn before_save(doc) {
    // Runs before every save
}

fn on_update(doc) {
    // Runs after update
}"
              class="w-full px-4 py-3 text-[13px] font-mono border border-border rounded-lg bg-surface text-text placeholder-text-light/60 focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-colors"
            />
          </div>
        </div>

      </div>
    </div>
  </div>
</template>
