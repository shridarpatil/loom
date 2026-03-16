<script setup lang="ts">
import { ref } from "vue";
import type { DocFieldMeta } from "@/utils/call";

const props = defineProps<{
  field: DocFieldMeta;
  modelValue: unknown;
  readOnly?: boolean;
  isImage?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: unknown];
}>();

const uploading = ref(false);
const error = ref("");

const currentUrl = ref((props.modelValue as string) || "");

async function onFileSelect(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

  uploading.value = true;
  error.value = "";

  try {
    const formData = new FormData();
    formData.append("file", file);

    const res = await fetch("/api/upload", {
      method: "POST",
      credentials: "include",
      body: formData,
    });

    if (!res.ok) {
      const err = await res.json();
      throw new Error(err.error || "Upload failed");
    }

    const data = await res.json();
    const url = data.data.file_url;
    currentUrl.value = url;
    emit("update:modelValue", url);
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Upload failed";
  } finally {
    uploading.value = false;
  }
}

function clear() {
  currentUrl.value = "";
  emit("update:modelValue", "");
}
</script>

<template>
  <div>
    <label class="block text-[12px] font-medium text-text-muted mb-1">
      {{ field.label || field.fieldname }}
      <span v-if="field.reqd" class="text-danger">*</span>
    </label>

    <!-- Image preview -->
    <div v-if="isImage && currentUrl" class="mb-2">
      <img
        :src="currentUrl"
        :alt="field.label || ''"
        class="max-w-[200px] max-h-[150px] rounded-md border border-border object-cover"
      />
    </div>

    <!-- Current value + clear -->
    <div v-if="currentUrl" class="flex items-center gap-2 mb-1.5">
      <a
        :href="currentUrl"
        target="_blank"
        class="text-[12px] text-primary-600 hover:underline truncate max-w-[300px]"
      >{{ currentUrl.split('/').pop() }}</a>
      <button
        v-if="!readOnly"
        class="text-[11px] text-text-light hover:text-danger"
        @click="clear"
      >Remove</button>
    </div>

    <!-- Upload input -->
    <div v-if="!readOnly" class="flex items-center gap-2">
      <label class="cursor-pointer px-3 py-1.5 text-[12px] font-medium border border-border rounded-md hover:bg-surface-muted transition-colors">
        <span v-if="uploading">Uploading...</span>
        <span v-else>{{ currentUrl ? 'Change file' : 'Choose file' }}</span>
        <input
          type="file"
          class="hidden"
          :accept="isImage ? 'image/*' : undefined"
          :disabled="readOnly || uploading"
          @change="onFileSelect"
        />
      </label>
    </div>

    <p v-if="error" class="mt-1 text-[11px] text-red-600">{{ error }}</p>
    <p v-if="field.description" class="mt-1 text-[11px] text-text-light">{{ field.description }}</p>
  </div>
</template>
