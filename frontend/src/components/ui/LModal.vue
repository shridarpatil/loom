<script setup lang="ts">
defineProps<{
  open: boolean;
  title: string;
}>();

defineEmits<{
  close: [];
}>();
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center">
      <div class="absolute inset-0 bg-black/20" @click="$emit('close')" />
      <div class="relative bg-white rounded-lg shadow-lg border border-border p-5 w-[360px]">
        <h3 class="text-[14px] font-semibold mb-1.5">{{ title }}</h3>
        <slot />
        <div v-if="$slots.footer" class="flex justify-end gap-1.5 mt-4">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
