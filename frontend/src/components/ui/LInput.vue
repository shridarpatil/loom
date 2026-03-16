<script setup lang="ts">
defineProps<{
  type?: string;
  placeholder?: string;
  disabled?: boolean;
  modelValue?: string | number;
}>();

defineEmits<{
  "update:modelValue": [value: string | number];
}>();
</script>

<template>
  <input
    :type="type || 'text'"
    :placeholder="placeholder"
    :disabled="disabled"
    :value="modelValue"
    class="w-full h-8 px-2.5 text-[13px] border border-border rounded-md bg-white text-text placeholder-text-light focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 disabled:bg-surface-raised disabled:text-text-muted disabled:cursor-not-allowed transition-colors"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
