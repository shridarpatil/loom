<script setup lang="ts">
import { useToast } from "@/composables/useToast";

const { toasts, remove } = useToast();

const iconPaths: Record<string, string> = {
  success: "M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
  error: "M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z",
  warning: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z",
  info: "m11.25 11.25.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z",
};

const colorMap: Record<string, { bg: string; icon: string; border: string }> = {
  success: { bg: "bg-emerald-50", icon: "text-emerald-500", border: "border-emerald-200" },
  error: { bg: "bg-red-50", icon: "text-red-500", border: "border-red-200" },
  warning: { bg: "bg-amber-50", icon: "text-amber-500", border: "border-amber-200" },
  info: { bg: "bg-blue-50", icon: "text-blue-500", border: "border-blue-200" },
};
</script>

<template>
  <Teleport to="body">
    <div class="fixed top-4 right-4 z-[100] flex flex-col gap-2 pointer-events-none">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        :class="[
          'pointer-events-auto flex items-start gap-2.5 px-4 py-3 rounded-lg border shadow-lg shadow-black/5 max-w-sm',
          colorMap[toast.type]?.bg || 'bg-white',
          colorMap[toast.type]?.border || 'border-border',
          toast.removing ? 'animate-toast-out' : 'animate-toast-in',
        ]"
      >
        <svg :class="['w-5 h-5 shrink-0 mt-0.5', colorMap[toast.type]?.icon]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" :d="iconPaths[toast.type]" />
        </svg>
        <p class="text-[13px] text-text flex-1 leading-snug">{{ toast.message }}</p>
        <button
          class="text-text-light hover:text-text-muted shrink-0 -mt-0.5"
          @click="remove(toast.id)"
        >
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  </Teleport>
</template>
