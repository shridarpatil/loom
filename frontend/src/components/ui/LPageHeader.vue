<script setup lang="ts">
import { onMounted, ref } from "vue";

defineProps<{
  title: string;
  subtitle?: string;
}>();

const mounted = ref(false);
onMounted(() => {
  mounted.value = true;
});
</script>

<template>
  <!-- Teleport page title into navbar left section -->
  <Teleport v-if="mounted" to="#navbar-page-header">
    <div class="flex items-center gap-2 min-w-0">
      <slot name="breadcrumb" />
      <div v-if="$slots.breadcrumb" class="text-gray-300">/</div>
      <h1 class="text-[15px] font-semibold tracking-tight truncate text-gray-800">{{ title }}</h1>
      <span v-if="subtitle" class="text-[12px] text-gray-400 truncate hidden sm:inline">{{ subtitle }}</span>
    </div>
  </Teleport>

  <!-- Teleport action buttons into navbar right section -->
  <Teleport v-if="mounted" to="#navbar-page-actions">
    <slot name="actions" />
  </Teleport>
</template>
