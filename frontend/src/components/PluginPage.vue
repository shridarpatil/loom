<script setup lang="ts">
import { ref, onMounted, shallowRef } from "vue";
import type { PluginPage } from "@/composables/usePlugins";

const props = defineProps<{
  page: PluginPage;
}>();

const error = ref("");
const loadedComponent = shallowRef<any>(null);

onMounted(async () => {
  try {
    // Load the external JS bundle
    const module = await import(/* @vite-ignore */ props.page.bundle);
    const component = module[props.page.component] || module.default;
    if (!component) {
      error.value = `Component "${props.page.component}" not found in bundle`;
      return;
    }
    loadedComponent.value = component;
  } catch (e) {
    error.value = `Failed to load plugin: ${e instanceof Error ? e.message : String(e)}`;
  }
});
</script>

<template>
  <div class="h-full">
    <div v-if="error" class="p-6">
      <div class="px-3 py-2 bg-red-50 border border-red-200 rounded-md text-[13px] text-red-700">
        {{ error }}
      </div>
    </div>
    <component v-else-if="loadedComponent" :is="loadedComponent" />
    <div v-else class="flex-1 flex items-center justify-center p-12">
      <svg class="w-5 h-5 animate-spin text-text-light" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
    </div>
  </div>
</template>
