<script setup lang="ts">
defineProps<{
  title: string;
  subtitle?: string;
}>();
</script>

<template>
  <div class="border-b border-border bg-white px-6 py-3 shrink-0">
    <div class="flex items-center justify-between">
      <div class="min-w-0">
        <slot name="breadcrumb" />
        <h1 class="text-base font-semibold truncate">{{ title }}</h1>
        <p v-if="subtitle" class="text-[12px] text-text-muted">{{ subtitle }}</p>
      </div>
      <div class="flex items-center gap-1.5 shrink-0">
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>
