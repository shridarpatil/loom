<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import ListView from "./ListView.vue";
import AppWorkspace from "./AppWorkspace.vue";

const props = defineProps<{ doctype: string }>();

const isApp = ref(false);
const checked = ref(false);

async function checkIfApp() {
  checked.value = false;
  try {
    const res = await fetch("/api/apps", { credentials: "include" });
    if (res.ok) {
      const data = await res.json();
      const apps = data.data || [];
      isApp.value = apps.some((a: any) => a.name === props.doctype);
    }
  } catch {
    isApp.value = false;
  }
  checked.value = true;
}

onMounted(checkIfApp);
watch(() => props.doctype, checkIfApp);
</script>

<template>
  <div v-if="!checked" class="flex-1 flex items-center justify-center h-full">
    <svg class="w-5 h-5 animate-spin text-text-light" fill="none" viewBox="0 0 24 24">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg>
  </div>
  <AppWorkspace v-else-if="isApp" :app-name="doctype" />
  <ListView v-else :doctype="doctype" />
</template>
