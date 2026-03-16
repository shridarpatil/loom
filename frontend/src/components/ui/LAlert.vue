<script setup lang="ts">
defineProps<{
  type?: "error" | "success" | "warning";
  dismissible?: boolean;
}>();

defineEmits<{
  dismiss: [];
}>();

const typeClasses: Record<string, string> = {
  error: "bg-red-50 border-red-200 text-red-700",
  success: "bg-emerald-50 border-emerald-200 text-emerald-700",
  warning: "bg-amber-50 border-amber-200 text-amber-700",
};
</script>

<template>
  <div
    :class="[
      'px-3 py-2 border rounded-md text-[13px] flex items-center gap-2',
      typeClasses[type || 'error'],
    ]"
  >
    <span class="flex-1"><slot /></span>
    <button
      v-if="dismissible"
      class="ml-auto opacity-60 hover:opacity-100 text-base leading-none"
      @click="$emit('dismiss')"
    >&times;</button>
  </div>
</template>
