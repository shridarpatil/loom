<script setup lang="ts">
defineProps<{
  color?: "blue" | "green" | "red" | "amber" | "gray";
  label: string;
}>();

const colorClasses: Record<string, string> = {
  blue: "bg-blue-50 text-blue-600",
  green: "bg-emerald-50 text-emerald-700",
  red: "bg-red-50 text-red-600",
  amber: "bg-amber-50 text-amber-700",
  gray: "bg-gray-50 text-gray-600",
};
</script>

<template>
  <span
    :class="[
      'inline-flex items-center px-1.5 py-0.5 rounded text-[11px] font-medium',
      colorClasses[color || 'gray'],
    ]"
  >{{ label }}</span>
</template>
