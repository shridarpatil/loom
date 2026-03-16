<script setup lang="ts">
defineProps<{
  open: boolean;
  title: string;
  size?: "sm" | "md" | "lg";
}>();

defineEmits<{
  close: [];
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/30 backdrop-blur-[2px] animate-fade-in" @click="$emit('close')" />

        <!-- Dialog -->
        <div
          :class="[
            'relative bg-white rounded-xl shadow-2xl shadow-black/10 border border-border/50 animate-scale-in',
            size === 'lg' ? 'w-[560px]' : size === 'sm' ? 'w-[320px]' : 'w-[420px]',
          ]"
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-5 pt-5 pb-0">
            <h3 class="text-[15px] font-semibold text-text">{{ title }}</h3>
            <button
              class="p-1 -mr-1 rounded-md text-text-light hover:text-text hover:bg-surface-raised transition-colors"
              @click="$emit('close')"
            >
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Body -->
          <div class="px-5 py-4">
            <slot />
          </div>

          <!-- Footer -->
          <div v-if="$slots.footer" class="flex items-center justify-end gap-2 px-5 pb-5 pt-0">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
