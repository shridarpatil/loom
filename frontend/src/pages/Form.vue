<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { loom, type DocTypeMeta, LoomApiError } from "@/utils/call";
import { socket } from "@/utils/socket";
import FormLayout from "@/components/FormLayout.vue";
import ActivityTimeline from "@/components/ActivityTimeline.vue";
import { LButton, LBadge, LAlert, LModal, LPageHeader } from "@/components/ui";
import { useSession } from "@/composables/useSession";

const props = defineProps<{
  doctype: string;
  id: string | null;
}>();

const router = useRouter();
const { roles: userRoles } = useSession();

const meta = ref<DocTypeMeta | null>(null);
const doc = ref<Record<string, unknown>>({});
const originalDoc = ref<Record<string, unknown>>({});
const loading = ref(true);
const saving = ref(false);
const error = ref("");
const success = ref("");
const showDeleteConfirm = ref(false);
const showSubmitConfirm = ref(false);
const showCancelConfirm = ref(false);

interface CustomButton {
  label: string;
  action: (doc: Record<string, unknown>) => void;
  variant?: string;
  view?: string;
}

const clientScriptFns = ref<{
  validate?: (doc: Record<string, unknown>) => string | undefined | void;
  on_change?: (doc: Record<string, unknown>, fieldname: string) => void;
}>({});
const customButtons = ref<CustomButton[]>([]);

function loadClientScript(script: string) {
  if (!script) { clientScriptFns.value = {}; customButtons.value = []; return; }
  try {
    const buttons: CustomButton[] = [];
    const sandbox: Record<string, unknown> = {
      // loom.add_button(label, action, options?)
      // options.view: "form" | "list" | "both" (default: "both")
      add_button(label: string, action: (doc: Record<string, unknown>) => void, options?: { variant?: string; view?: string }) {
        buttons.push({ label, action, variant: options?.variant || "secondary", view: options?.view || "both" });
      },
    };
    const fn = new Function("loom", script);
    fn(sandbox);
    clientScriptFns.value = {
      validate: sandbox.validate as typeof clientScriptFns.value.validate,
      on_change: sandbox.on_change as typeof clientScriptFns.value.on_change,
    };
    // Only show buttons meant for form view
    customButtons.value = buttons.filter((b) => b.view === "form" || b.view === "both");
  } catch (e) {
    console.error("Client script error:", e);
    clientScriptFns.value = {};
    customButtons.value = [];
  }
}

function runCustomButton(btn: CustomButton) {
  try {
    btn.action(doc.value);
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Button action failed";
  }
}

async function onFieldChange(newDoc: Record<string, unknown>) {
  const prev = doc.value;
  doc.value = newDoc;
  const changedFields: string[] = [];
  for (const key of Object.keys(newDoc)) {
    if (newDoc[key] !== prev[key]) changedFields.push(key);
  }

  if (meta.value) {
    for (const changedField of changedFields) {
      const changedMeta = meta.value.fields.find((f) => f.fieldname === changedField);
      if (changedMeta?.fieldtype === "Link" && changedMeta.options) {
        const linkValue = newDoc[changedField] as string;
        const fetchFields = meta.value.fields.filter(
          (f) => f.fetch_from && f.fetch_from.startsWith(changedField + "."),
        );
        if (fetchFields.length > 0) {
          if (linkValue) {
            try {
              const res = await loom.resource(changedMeta.options).get(linkValue);
              const linkedDoc = res.data;
              const updates: Record<string, unknown> = { ...doc.value };
              for (const ff of fetchFields) {
                const sourceField = ff.fetch_from!.split(".").slice(1).join(".");
                updates[ff.fieldname] = linkedDoc[sourceField] ?? null;
              }
              doc.value = updates;
            } catch { /* linked doc may not exist */ }
          } else {
            const updates: Record<string, unknown> = { ...doc.value };
            for (const ff of fetchFields) updates[ff.fieldname] = null;
            doc.value = updates;
          }
        }
      }
    }
  }

  if (clientScriptFns.value.on_change) {
    for (const key of changedFields) {
      try { clientScriptFns.value.on_change(doc.value, key); } catch (e) { console.error("on_change error:", e); }
    }
  }
}

const isNew = computed(() => !props.id);
const title = computed(() => {
  if (isNew.value) return `New ${props.doctype}`;
  if (meta.value?.title_field && doc.value[meta.value.title_field]) return String(doc.value[meta.value.title_field]);
  return String(props.id);
});
const isDirty = computed(() => JSON.stringify(doc.value) !== JSON.stringify(originalDoc.value));

const isSubmittable = computed(() => meta.value?.is_submittable ?? false);
const docstatus = computed(() => {
  const ds = doc.value.docstatus;
  if (typeof ds === "number") return ds;
  if (typeof ds === "string") return parseInt(ds) || 0;
  return 0;
});
const isSubmitted = computed(() => docstatus.value === 1);
const isCancelled = computed(() => docstatus.value === 2);
const docstatusLabel = computed(() => ({ 0: "Draft", 1: "Submitted", 2: "Cancelled" }[docstatus.value] || ""));
const docstatusColor = computed(() => ({
  0: "amber" as const,
  1: "blue" as const,
  2: "red" as const,
}[docstatus.value] || "gray" as const));

const canSave = computed(() => !(isSubmitted.value || isCancelled.value) && (isNew.value || isDirty.value));
const canDelete = computed(() => !isNew.value && !isSubmitted.value);
const canSubmit = computed(() => isSubmittable.value && docstatus.value === 0 && !isDirty.value && !isNew.value);
const canCancel = computed(() => isSubmittable.value && docstatus.value === 1);
const formReadOnly = computed(() => isSubmitted.value || isCancelled.value);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const metaRes = await loom.getMeta(props.doctype);
    meta.value = metaRes.data;
    loadClientScript((metaRes.data as any).client_script || "");
    if (!isNew.value && props.id) {
      const docRes = await loom.resource(props.doctype).get(props.id);
      doc.value = { ...docRes.data };
      originalDoc.value = { ...docRes.data };
    } else {
      const defaults: Record<string, unknown> = {};
      for (const field of meta.value.fields) { if (field.default) defaults[field.fieldname] = field.default; }
      doc.value = defaults;
      originalDoc.value = {};
    }
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Failed to load";
  } finally {
    loading.value = false;
  }
}

async function save() {
  saving.value = true; error.value = ""; success.value = "";
  if (clientScriptFns.value.validate) {
    try {
      const validationError = clientScriptFns.value.validate(doc.value);
      if (validationError) { error.value = validationError; saving.value = false; return; }
    } catch (e) { error.value = e instanceof Error ? e.message : "Validation error"; saving.value = false; return; }
  }
  try {
    if (isNew.value) {
      const res = await loom.resource(props.doctype).insert(doc.value);
      success.value = `${props.doctype} created`;
      router.replace(`/app/${props.doctype}/${res.data.id as string}`);
    } else if (props.id) {
      const changes: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(doc.value)) { if (val !== originalDoc.value[key]) changes[key] = val; }
      if (Object.keys(changes).length === 0) { success.value = "No changes"; return; }
      const res = await loom.resource(props.doctype).update(props.id, changes);
      doc.value = { ...res.data }; originalDoc.value = { ...res.data };
      success.value = "Saved";
    }
  } catch (e: unknown) {
    error.value = e instanceof LoomApiError ? e.message : (e instanceof Error ? e.message : "Save failed");
  } finally {
    saving.value = false;
    if (success.value) setTimeout(() => (success.value = ""), 3000);
  }
}

async function submitDoc() {
  if (!props.id) return;
  showSubmitConfirm.value = false; saving.value = true; error.value = ""; success.value = "";
  try {
    const res = await loom.resource(props.doctype).submit(props.id);
    doc.value = { ...res.data }; originalDoc.value = { ...res.data }; success.value = "Submitted";
  } catch (e: unknown) {
    error.value = e instanceof LoomApiError ? e.message : (e instanceof Error ? e.message : "Submit failed");
  } finally { saving.value = false; if (success.value) setTimeout(() => (success.value = ""), 3000); }
}

async function cancelDoc() {
  if (!props.id) return;
  showCancelConfirm.value = false; saving.value = true; error.value = ""; success.value = "";
  try {
    const res = await loom.resource(props.doctype).cancel(props.id);
    doc.value = { ...res.data }; originalDoc.value = { ...res.data }; success.value = "Cancelled";
  } catch (e: unknown) {
    error.value = e instanceof LoomApiError ? e.message : (e instanceof Error ? e.message : "Cancel failed");
  } finally { saving.value = false; if (success.value) setTimeout(() => (success.value = ""), 3000); }
}

async function deleteDoc() {
  if (!props.id) return;
  try { await loom.resource(props.doctype).delete(props.id); router.push(`/app/${props.doctype}`); }
  catch (e: unknown) { error.value = e instanceof Error ? e.message : "Delete failed"; showDeleteConfirm.value = false; }
}

watch(() => [props.doctype, props.id], () => load(), { immediate: true });

// Realtime: reload when this doc is updated by another user/tab
function onDocUpdate(data: unknown) {
  const d = data as { doctype?: string; name?: string; action?: string };
  if (d.doctype === props.doctype && d.name === props.id && !saving.value) {
    // Silently reload — don't overwrite if user has unsaved changes
    if (!isDirty.value) {
      load();
    }
  }
}
onMounted(() => socket.on("doc_update", onDocUpdate));
onUnmounted(() => socket.off("doc_update", onDocUpdate));
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <LPageHeader :title="title">
      <template #breadcrumb>
        <button
          class="inline-flex items-center gap-0.5 text-[12px] text-text-muted hover:text-primary-600 transition-colors"
          @click="router.push(`/app/${doctype}`)"
        >
          <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5" />
          </svg>
          {{ doctype }}
        </button>
      </template>
      <template #actions>
        <!-- Custom buttons from client script -->
        <LButton
          v-for="(btn, i) in customButtons"
          :key="'cb-' + i"
          :variant="(btn.variant as any) || 'secondary'"
          size="sm"
          @click="runCustomButton(btn)"
        >{{ btn.label }}</LButton>

        <LBadge
          v-if="isSubmittable && !isNew"
          :color="docstatusColor"
          :label="docstatusLabel"
        />
        <LButton v-if="canCancel" variant="danger" size="sm" @click="showCancelConfirm = true">
          Cancel
        </LButton>
        <LButton v-if="canDelete" variant="secondary" @click="showDeleteConfirm = true">
          Delete
        </LButton>
        <LButton v-if="canSubmit" @click="showSubmitConfirm = true">
          Submit
        </LButton>
        <LButton
          v-if="!isSubmitted && !isCancelled"
          :disabled="saving || !canSave"
          :loading="saving"
          @click="save"
        >
          {{ isNew ? "Create" : "Save" }}
        </LButton>
      </template>
    </LPageHeader>

    <!-- Alerts -->
    <div v-if="error || success" class="px-6 pt-3 space-y-1.5">
      <LAlert v-if="error" type="error" dismissible @dismiss="error = ''">{{ error }}</LAlert>
      <LAlert v-if="success" type="success">{{ success }}</LAlert>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <svg class="w-5 h-5 animate-spin text-text-light" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
    </div>

    <!-- Form -->
    <div v-else-if="meta" class="flex-1 overflow-auto">
      <div class="px-6 py-5">
        <!-- Doc info -->
        <div v-if="!isNew" class="flex items-center gap-4 mb-5 px-1 text-[11px] text-text-light">
          <span class="flex items-center gap-1">
            <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M5.25 8.25h15m-16.5 7.5h15m-1.8-13.5-3.9 19.5m-2.1-19.5-3.9 19.5" /></svg>
            <span class="font-mono text-text-muted">{{ props.id }}</span>
          </span>
          <span v-if="doc.owner" class="flex items-center gap-1">
            <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 6a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0ZM4.501 20.118a7.5 7.5 0 0 1 14.998 0" /></svg>
            <span class="text-text-muted">{{ doc.owner }}</span>
          </span>
          <span v-if="doc.modified" class="flex items-center gap-1">
            <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
            <span class="text-text-muted">{{ String(doc.modified).slice(0, 19) }}</span>
          </span>
        </div>

        <FormLayout
          :fields="meta.fields"
          :modelValue="doc"
          :readOnly="formReadOnly"
          :permissions="meta.permissions"
          :userRoles="userRoles"
          @update:modelValue="onFieldChange"
        />

        <!-- Activity Timeline -->
        <ActivityTimeline
          v-if="!isNew && props.id"
          :doctype="props.doctype"
          :docname="props.id"
        />
      </div>
    </div>

    <!-- Delete Modal -->
    <LModal :open="showDeleteConfirm" :title="`Delete ${doctype}`" @close="showDeleteConfirm = false">
      <p class="text-[13px] text-text-muted">
        Are you sure you want to delete <strong class="text-text">{{ props.id }}</strong>? This cannot be undone.
      </p>
      <template #footer>
        <LButton variant="secondary" @click="showDeleteConfirm = false">No</LButton>
        <LButton variant="danger" @click="deleteDoc">Delete</LButton>
      </template>
    </LModal>

    <!-- Submit Modal -->
    <LModal :open="showSubmitConfirm" :title="`Submit ${doctype}`" @close="showSubmitConfirm = false">
      <p class="text-[13px] text-text-muted">
        Once submitted, this document cannot be edited. Continue?
      </p>
      <template #footer>
        <LButton variant="secondary" @click="showSubmitConfirm = false">No</LButton>
        <LButton @click="submitDoc">Submit</LButton>
      </template>
    </LModal>

    <!-- Cancel Modal -->
    <LModal :open="showCancelConfirm" :title="`Cancel ${doctype}`" @close="showCancelConfirm = false">
      <p class="text-[13px] text-text-muted">
        Are you sure you want to cancel <strong class="text-text">{{ props.id }}</strong>?
      </p>
      <template #footer>
        <LButton variant="secondary" @click="showCancelConfirm = false">No</LButton>
        <LButton variant="danger" @click="cancelDoc">Cancel Document</LButton>
      </template>
    </LModal>
  </div>
</template>
