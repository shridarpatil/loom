<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useRouter } from "vue-router";
import { loom, type DocPermMeta } from "@/utils/call";
import { LButton, LAlert, LPageHeader } from "@/components/ui";

const props = defineProps<{ doctype?: string }>();
const router = useRouter();

// --- Shared state ---
const viewMode = ref<"doctype" | "role">(props.doctype ? "doctype" : "doctype");
const doctypes = ref<string[]>([]);
const availableRoles = ref<string[]>([]);
const loading = ref(false);
const saving = ref(false);
const error = ref("");
const success = ref("");

function showSuccess(msg: string) {
  success.value = msg;
  setTimeout(() => (success.value = ""), 3000);
}

// =========================================================================
// By DocType view
// =========================================================================
const selectedDoctype = ref(props.doctype || "");
const isSubmittable = ref(false);
const defaultPermissions = ref<DocPermMeta[]>([]);
const isOverridden = ref(false);
const dtPermissions = ref<DocPermMeta[]>([]);
const dtOriginalJson = ref("");

const dtHasChanges = computed(() => JSON.stringify(dtPermissions.value) !== dtOriginalJson.value);

const dtPermColumns = computed(() => {
  const cols = ["read", "write", "create", "delete"];
  if (isSubmittable.value) cols.push("submit", "cancel");
  return cols;
});

async function loadDtPermissions() {
  if (!selectedDoctype.value) return;
  loading.value = true;
  error.value = "";
  try {
    const res = await fetch(`/api/role-permission/${encodeURIComponent(selectedDoctype.value)}`, {
      credentials: "include",
    });
    if (!res.ok) throw new Error("Failed to load permissions");
    const data = (await res.json()).data;

    isSubmittable.value = data.is_submittable || false;
    defaultPermissions.value = data.default_permissions || [];
    availableRoles.value = data.roles || [];
    isOverridden.value = data.override_permissions != null;

    // Only show override rules — not defaults
    if (isOverridden.value && Array.isArray(data.override_permissions)) {
      dtPermissions.value = (data.override_permissions as DocPermMeta[]).map((p) => ({ ...p }));
    } else {
      dtPermissions.value = [];
    }
    dtOriginalJson.value = JSON.stringify(dtPermissions.value);
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Failed to load";
  } finally {
    loading.value = false;
  }
}

function dtAddRule() {
  dtPermissions.value.push({
    role: availableRoles.value[0] || "All",
    permlevel: 0, read: true, write: false, create: false,
    delete: false, submit: false, cancel: false,
  });
}

function dtRemoveRule(i: number) { dtPermissions.value.splice(i, 1); }

async function dtSave() {
  saving.value = true; error.value = "";
  try {
    if (dtPermissions.value.length === 0) {
      // No overrides — remove the override entry
      await fetch(`/api/role-permission/${encodeURIComponent(selectedDoctype.value)}`, {
        method: "DELETE", credentials: "include",
      });
      isOverridden.value = false;
      dtOriginalJson.value = JSON.stringify(dtPermissions.value);
      showSuccess("All overrides removed — using defaults");
      return;
    }
    const res = await fetch(`/api/role-permission/${encodeURIComponent(selectedDoctype.value)}`, {
      method: "PUT", headers: { "Content-Type": "application/json" },
      credentials: "include",
      body: JSON.stringify({ permissions: dtPermissions.value }),
    });
    if (!res.ok) { const err = await res.json(); throw new Error(err.error || "Save failed"); }
    isOverridden.value = true;
    dtOriginalJson.value = JSON.stringify(dtPermissions.value);
    showSuccess("Permissions saved");
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Save failed";
  } finally { saving.value = false; }
}

async function dtReset() {
  if (!confirm("Reset permissions to the defaults shipped with the app?")) return;
  saving.value = true; error.value = "";
  try {
    await fetch(`/api/role-permission/${encodeURIComponent(selectedDoctype.value)}`, {
      method: "DELETE", credentials: "include",
    });
    isOverridden.value = false;
    dtPermissions.value = [];
    dtOriginalJson.value = JSON.stringify(dtPermissions.value);
    showSuccess("All overrides removed");
  } catch { error.value = "Reset failed"; }
  finally { saving.value = false; }
}

function onDoctypeChange(dt: string) {
  selectedDoctype.value = dt;
  if (dt) router.replace(`/app/role-permission-manager/${dt}`);
}

// =========================================================================
// By Role view
// =========================================================================
interface RolePermEntry {
  doctype: string;
  is_submittable: boolean;
  role: string;
  permlevel: number;
  read: boolean;
  write: boolean;
  create: boolean;
  delete: boolean;
  submit: boolean;
  cancel: boolean;
  if_owner: boolean;
}

const selectedRole = ref("");
const roleEntries = ref<RolePermEntry[]>([]);
const roleOriginalJson = ref("");

const roleHasChanges = computed(() => JSON.stringify(roleEntries.value) !== roleOriginalJson.value);

async function loadRolePermissions() {
  if (!selectedRole.value) return;
  loading.value = true; error.value = "";
  try {
    const res = await fetch(`/api/role-permission-by-role/${encodeURIComponent(selectedRole.value)}`, {
      credentials: "include",
    });
    if (!res.ok) throw new Error("Failed to load");
    const data = (await res.json()).data;
    availableRoles.value = data.roles || [];
    roleEntries.value = (data.permissions || []).map((e: RolePermEntry) => ({ ...e }));
    roleOriginalJson.value = JSON.stringify(roleEntries.value);
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Failed to load";
  } finally { loading.value = false; }
}

async function roleSave() {
  // Group changes by doctype and save each one
  saving.value = true; error.value = "";
  try {
    // Build a map: doctype -> full permission list (we need to load the current perms for each affected doctype, swap in the edited rows, and save)
    const affectedDoctypes = new Set(roleEntries.value.map((e) => e.doctype));
    for (const dt of affectedDoctypes) {
      // Load current effective permissions for this doctype
      const res = await fetch(`/api/role-permission/${encodeURIComponent(dt)}`, { credentials: "include" });
      if (!res.ok) continue;
      const data = (await res.json()).data;
      const currentPerms: DocPermMeta[] = data.override_permissions ?? data.default_permissions ?? [];

      // Remove all rules for the selected role, then add back the edited ones
      const otherPerms = currentPerms.filter((p) => p.role !== selectedRole.value);
      const editedForDt = roleEntries.value
        .filter((e) => e.doctype === dt)
        .map(({ doctype, is_submittable, ...perm }) => perm as DocPermMeta);
      const merged = [...otherPerms, ...editedForDt];

      await fetch(`/api/role-permission/${encodeURIComponent(dt)}`, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ permissions: merged }),
      });
    }
    roleOriginalJson.value = JSON.stringify(roleEntries.value);
    showSuccess("Permissions saved across all DocTypes");
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Save failed";
  } finally { saving.value = false; }
}

// --- Init ---
async function loadDoctypes() {
  try {
    const res = await loom.resource("DocType").getList({ limit: 200 });
    doctypes.value = (res.data as Array<{ name: string }>)
      .map((d) => d.name).filter((n) => n !== "DocType").sort();
  } catch { doctypes.value = []; }
}

// Load roles for the role-view selector (even before a role is selected)
async function loadRoles() {
  // Roles are loaded when a doctype or role is selected via their respective APIs
}

watch(() => selectedDoctype.value, (v) => { if (v && viewMode.value === "doctype") loadDtPermissions(); });
watch(() => selectedRole.value, (v) => { if (v && viewMode.value === "role") loadRolePermissions(); });

loadDoctypes();
loadRoles();
if (props.doctype) loadDtPermissions();
</script>

<template>
  <div class="h-full flex flex-col">
    <LPageHeader title="Role Permission Manager">
      <template #actions>
        <template v-if="viewMode === 'doctype'">
          <LButton v-if="isOverridden" variant="secondary" size="sm" :disabled="saving" @click="dtReset">Reset to Defaults</LButton>
          <LButton :disabled="saving || !dtHasChanges || !selectedDoctype" :loading="saving" @click="dtSave">Save</LButton>
        </template>
        <template v-else>
          <LButton :disabled="saving || !roleHasChanges || !selectedRole" :loading="saving" @click="roleSave">Save</LButton>
        </template>
      </template>
    </LPageHeader>

    <div v-if="error || success" class="px-6 pt-3 space-y-1.5">
      <LAlert v-if="error" type="error" dismissible @dismiss="error = ''">{{ error }}</LAlert>
      <LAlert v-if="success" type="success">{{ success }}</LAlert>
    </div>

    <div class="px-6 py-4 flex-1 overflow-auto">
      <!-- View toggle + selector row -->
      <div class="mb-4 flex items-center gap-3 flex-wrap">
        <!-- Tab toggle -->
        <div class="flex rounded-lg border border-border overflow-hidden">
          <button
            :class="['px-3 py-1.5 text-[12px] font-medium transition-colors', viewMode === 'doctype' ? 'bg-primary-50 text-primary-700' : 'bg-white text-text-muted hover:bg-surface-muted']"
            @click="viewMode = 'doctype'"
          >By DocType</button>
          <button
            :class="['px-3 py-1.5 text-[12px] font-medium transition-colors border-l border-border', viewMode === 'role' ? 'bg-primary-50 text-primary-700' : 'bg-white text-text-muted hover:bg-surface-muted']"
            @click="viewMode = 'role'"
          >By Role</button>
        </div>

        <!-- Selector -->
        <template v-if="viewMode === 'doctype'">
          <select
            :value="selectedDoctype"
            class="px-3 py-[7px] text-sm border border-border rounded-lg bg-white text-text min-w-[240px] focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400"
            @change="onDoctypeChange(($event.target as HTMLSelectElement).value)"
          >
            <option value="">Select a DocType...</option>
            <option v-for="dt in doctypes" :key="dt" :value="dt">{{ dt }}</option>
          </select>
          <span v-if="isOverridden" class="px-2 py-0.5 text-[10px] font-medium bg-amber-50 text-amber-700 rounded-md border border-amber-200">Customized</span>
        </template>
        <template v-else>
          <select
            v-model="selectedRole"
            class="px-3 py-[7px] text-sm border border-border rounded-lg bg-white text-text min-w-[240px] focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400"
          >
            <option value="">Select a Role...</option>
            <option v-for="role in availableRoles" :key="role" :value="role">{{ role }}</option>
          </select>
        </template>
      </div>

      <!-- Loading -->
      <div v-if="loading" class="py-12 text-center text-text-light text-[13px]">Loading permissions...</div>

      <!-- =============== BY DOCTYPE VIEW =============== -->
      <template v-else-if="viewMode === 'doctype'">
        <div v-if="!selectedDoctype" class="py-12 text-center text-text-light text-[13px]">
          Select a DocType to manage its permissions.
        </div>
        <div v-else class="bg-white border border-border rounded-lg overflow-hidden">
          <table class="w-full">
            <thead>
              <tr class="border-b border-border">
                <th class="text-left px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[180px]">Role</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[50px]">Level</th>
                <th v-for="col in dtPermColumns" :key="col" class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">{{ col }}</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">if_owner</th>
                <th class="w-10 bg-surface-muted/50"></th>
              </tr>
            </thead>
            <tbody class="text-[13px]">
              <tr v-if="dtPermissions.length === 0">
                <td :colspan="dtPermColumns.length + 4" class="px-3 py-8 text-center text-text-light">
                  No permission overrides. Using defaults from DocType definition.<br />
                  <span class="text-[11px]">Click "Add Rule" to add a permission override.</span>
                </td>
              </tr>
              <tr v-for="(perm, pi) in dtPermissions" :key="pi" class="border-b border-border last:border-0 hover:bg-surface-muted/30 group">
                <td class="px-3 py-2">
                  <select v-model="perm.role" class="w-full px-2 py-1 text-[12px] border border-border rounded bg-white text-text focus:outline-none focus:ring-1 focus:ring-primary-500/30">
                    <option v-for="role in availableRoles" :key="role" :value="role">{{ role }}</option>
                  </select>
                </td>
                <td class="px-2 py-2 text-center">
                  <input v-model.number="perm.permlevel" type="number" min="0" max="9" class="w-10 px-1 py-1 text-[12px] text-center border border-border rounded bg-white text-text focus:outline-none focus:ring-1 focus:ring-primary-500/30" />
                </td>
                <td v-for="col in dtPermColumns" :key="col" class="px-2 py-2 text-center">
                  <input type="checkbox" :checked="(perm as any)[col]" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" @change="(perm as any)[col] = ($event.target as HTMLInputElement).checked" />
                </td>
                <td class="px-2 py-2 text-center">
                  <input v-model="perm.if_owner" type="checkbox" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" />
                </td>
                <td class="px-2 py-2 text-center">
                  <button class="p-1 text-text-light hover:text-danger opacity-0 group-hover:opacity-100 transition-all" @click="dtRemoveRule(pi)">
                    <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <div class="px-3 py-2 border-t border-border bg-surface-muted/30">
            <LButton variant="secondary" size="sm" @click="dtAddRule">
              <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
              Add Rule
            </LButton>
          </div>
        </div>
      </template>

      <!-- =============== BY ROLE VIEW =============== -->
      <template v-else>
        <div v-if="!selectedRole" class="py-12 text-center text-text-light text-[13px]">
          Select a Role to see its permissions across all DocTypes.
        </div>
        <div v-else-if="roleEntries.length === 0" class="py-12 text-center text-text-light text-[13px]">
          This role has no permissions on any DocType.
        </div>
        <div v-else class="bg-white border border-border rounded-lg overflow-hidden">
          <table class="w-full">
            <thead>
              <tr class="border-b border-border">
                <th class="text-left px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[200px]">DocType</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[50px]">Level</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">Read</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">Write</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">Create</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">Delete</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">Submit</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">Cancel</th>
                <th class="text-center px-2 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-light bg-surface-muted/50 w-[70px]">if_owner</th>
              </tr>
            </thead>
            <tbody class="text-[13px]">
              <tr v-for="(entry, ei) in roleEntries" :key="ei" class="border-b border-border last:border-0 hover:bg-surface-muted/30">
                <td class="px-3 py-2">
                  <button class="text-primary-600 hover:underline text-[12px] font-medium" @click="viewMode = 'doctype'; onDoctypeChange(entry.doctype)">{{ entry.doctype }}</button>
                </td>
                <td class="px-2 py-2 text-center text-[12px] text-text-muted">{{ entry.permlevel }}</td>
                <td class="px-2 py-2 text-center"><input type="checkbox" v-model="entry.read" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></td>
                <td class="px-2 py-2 text-center"><input type="checkbox" v-model="entry.write" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></td>
                <td class="px-2 py-2 text-center"><input type="checkbox" v-model="entry.create" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></td>
                <td class="px-2 py-2 text-center"><input type="checkbox" v-model="entry.delete" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></td>
                <td class="px-2 py-2 text-center">
                  <input v-if="entry.is_submittable" type="checkbox" v-model="entry.submit" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" />
                  <span v-else class="text-text-light">-</span>
                </td>
                <td class="px-2 py-2 text-center">
                  <input v-if="entry.is_submittable" type="checkbox" v-model="entry.cancel" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" />
                  <span v-else class="text-text-light">-</span>
                </td>
                <td class="px-2 py-2 text-center"><input type="checkbox" v-model="entry.if_owner" class="w-4 h-4 rounded border-border-strong text-primary-600 focus:ring-primary-500" /></td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>
    </div>
  </div>
</template>
