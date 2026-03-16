<script setup lang="ts">
import { useRouter } from "vue-router";

const props = defineProps<{
  widget: {
    label: string;
    route: string;
    icon?: string;
    color?: string;
  };
}>();

const router = useRouter();
</script>

<template>
  <button
    class="bg-white border border-border rounded-lg px-4 py-3 text-left hover:border-primary-300 transition-all group"
    @click="router.push(widget.route)"
  >
    <div
      class="w-7 h-7 rounded flex items-center justify-center mb-2 transition-colors"
      :class="widget.color ? '' : 'bg-primary-50 group-hover:bg-primary-100'"
      :style="widget.color ? { backgroundColor: widget.color + '20', color: widget.color } : {}"
    >
      <svg class="w-4 h-4 text-primary-600" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
        <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
      </svg>
    </div>
    <div class="text-[13px] font-medium">{{ widget.label }}</div>
  </button>
</template>
