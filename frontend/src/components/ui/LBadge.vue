<script setup lang="ts">
defineProps<{
  color?: "blue" | "green" | "red" | "amber" | "gray";
  label: string;
}>();

const colorClasses: Record<string, string> = {
  blue: "bg-blue-50 text-blue-600 ring-blue-500/10",
  green: "bg-emerald-50 text-emerald-700 ring-emerald-600/10",
  red: "bg-red-50 text-red-600 ring-red-500/10",
  amber: "bg-amber-50 text-amber-700 ring-amber-600/10",
  gray: "bg-gray-50 text-gray-600 ring-gray-500/10",
};
</script>

<template>
  <span
    :class="[
      'inline-flex items-center px-2 py-0.5 rounded-md text-[11px] font-semibold ring-1 ring-inset',
      colorClasses[color || 'gray'],
    ]"
  >{{ label }}</span>
</template>
