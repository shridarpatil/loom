<script setup lang="ts">
defineProps<{
  variant?: "primary" | "secondary" | "danger" | "ghost";
  size?: "sm" | "md";
  loading?: boolean;
  disabled?: boolean;
}>();
</script>

<template>
  <button
    :class="[
      'inline-flex items-center justify-center gap-1.5 font-medium rounded-lg transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer select-none',
      size === 'sm' ? 'px-2.5 py-1 text-[12px]' : 'px-3.5 py-[7px] text-[13px]',
      variant === 'danger'
        ? 'bg-red-600 text-white hover:bg-red-700 active:bg-red-800 shadow-sm'
        : variant === 'ghost'
          ? 'text-text-muted hover:bg-surface-raised hover:text-text active:bg-surface-muted'
          : variant === 'secondary'
            ? 'border border-border bg-white text-text-muted hover:bg-surface-muted hover:text-text hover:border-border-strong active:bg-surface-raised shadow-sm shadow-black/[0.02]'
            : 'bg-primary-600 text-white hover:bg-primary-700 active:bg-primary-800 shadow-sm shadow-primary-600/20',
    ]"
    :disabled="disabled || loading"
  >
    <svg v-if="loading" class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg>
    <slot />
  </button>
</template>
