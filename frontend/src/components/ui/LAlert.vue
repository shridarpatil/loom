<script setup lang="ts">
defineProps<{
  type?: "error" | "success" | "warning";
  dismissible?: boolean;
}>();

defineEmits<{
  dismiss: [];
}>();

const config: Record<string, { bg: string; icon: string; path: string }> = {
  error: {
    bg: "bg-red-50 border-red-200 text-red-800",
    icon: "text-red-500",
    path: "M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z",
  },
  success: {
    bg: "bg-emerald-50 border-emerald-200 text-emerald-800",
    icon: "text-emerald-500",
    path: "M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
  },
  warning: {
    bg: "bg-amber-50 border-amber-200 text-amber-800",
    icon: "text-amber-500",
    path: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z",
  },
};
</script>

<template>
  <div
    :class="[
      'px-3.5 py-2.5 border rounded-lg text-[13px] flex items-start gap-2.5 animate-slide-down',
      config[type || 'error']?.bg,
    ]"
  >
    <svg :class="['w-4 h-4 shrink-0 mt-0.5', config[type || 'error']?.icon]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
      <path stroke-linecap="round" stroke-linejoin="round" :d="config[type || 'error']?.path" />
    </svg>
    <span class="flex-1 leading-snug"><slot /></span>
    <button
      v-if="dismissible"
      class="ml-auto opacity-50 hover:opacity-100 shrink-0"
      @click="$emit('dismiss')"
    >
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
      </svg>
    </button>
  </div>
</template>
