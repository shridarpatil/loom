<script setup lang="ts">
defineProps<{
  title: string;
  subtitle?: string;
}>();
</script>

<template>
  <div class="sticky top-0 z-10 border-b border-border bg-white/80 backdrop-blur-xl px-6 py-3 shrink-0">
    <div class="flex items-center justify-between gap-4">
      <div class="min-w-0">
        <slot name="breadcrumb" />
        <h1 class="text-[16px] font-semibold tracking-tight truncate text-text">{{ title }}</h1>
        <p v-if="subtitle" class="text-[12px] text-text-muted mt-0.5">{{ subtitle }}</p>
      </div>
      <div class="flex items-center gap-2 shrink-0">
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>
