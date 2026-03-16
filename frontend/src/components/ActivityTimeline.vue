<script setup lang="ts">
import { ref, onMounted } from "vue";
import { loom, type ActivityEntry } from "@/utils/call";

const props = defineProps<{
  doctype: string;
  docname: string;
}>();

const entries = ref<ActivityEntry[]>([]);
const loading = ref(true);
const commentText = ref("");
const posting = ref(false);

const actionConfig: Record<string, { color: string; icon: string }> = {
  Created: { color: "text-emerald-500", icon: "M12 4.5v15m7.5-7.5h-15" },
  Updated: { color: "text-blue-500", icon: "M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" },
  Submitted: { color: "text-blue-600", icon: "m4.5 12.75 6 6 9-13.5" },
  Cancelled: { color: "text-red-500", icon: "M6 18 18 6M6 6l12 12" },
  Commented: { color: "text-gray-500", icon: "M7.5 8.25h9m-9 3H12m-9.75 1.51c0 1.6 1.123 2.994 2.707 3.227 1.087.16 2.185.283 3.293.369V21l4.076-4.076a1.526 1.526 0 0 1 1.037-.443 48.282 48.282 0 0 0 5.68-.494c1.584-.233 2.707-1.626 2.707-3.228V6.741c0-1.602-1.123-2.995-2.707-3.228A48.394 48.394 0 0 0 12 3c-2.392 0-4.744.175-7.043.513C3.373 3.746 2.25 5.14 2.25 6.741v6.018Z" },
};

async function loadActivity() {
  loading.value = true;
  try {
    const res = await loom.activity(props.doctype, props.docname).get();
    entries.value = res.data;
  } catch {
    entries.value = [];
  } finally {
    loading.value = false;
  }
}

async function postComment() {
  if (!commentText.value.trim()) return;
  posting.value = true;
  try {
    await loom.activity(props.doctype, props.docname).comment(commentText.value.trim());
    commentText.value = "";
    await loadActivity();
  } catch {
    // silently fail
  } finally {
    posting.value = false;
  }
}

function relativeTime(timestamp: string): string {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
  return timestamp.slice(0, 10);
}

function getConfig(action: string) {
  return actionConfig[action] || actionConfig.Updated;
}

function formatChangeValue(val: unknown): string {
  if (val === null || val === undefined || val === "") return "(empty)";
  if (typeof val === "boolean") return val ? "Yes" : "No";
  return String(val);
}

onMounted(loadActivity);
</script>

<template>
  <div class="mt-6 border-t border-border pt-4">
    <h3 class="text-[12px] font-semibold text-text-light uppercase tracking-wider mb-3">Activity</h3>

    <!-- Comment input -->
    <div class="flex items-start gap-2 mb-4">
      <textarea
        v-model="commentText"
        placeholder="Add a comment..."
        rows="2"
        class="flex-1 px-3 py-2 text-[13px] border border-border rounded-lg bg-white text-text placeholder-text-light focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 resize-none"
      />
      <button
        :disabled="posting || !commentText.trim()"
        class="px-3 py-2 text-[12px] font-medium text-white bg-primary-600 rounded-lg hover:bg-primary-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
        @click="postComment"
      >Comment</button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center gap-2 text-[12px] text-text-light py-2">
      <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
      Loading activity...
    </div>

    <!-- Empty -->
    <div v-else-if="entries.length === 0" class="text-[12px] text-text-light py-2">
      No activity yet.
    </div>

    <!-- Timeline -->
    <div v-else class="relative">
      <!-- Vertical line -->
      <div class="absolute left-[9px] top-2 bottom-2 w-px bg-border" />

      <div v-for="entry in entries" :key="entry.id" class="relative flex items-start gap-3 pb-3">
        <!-- Dot/icon -->
        <div class="relative z-10 w-[20px] h-[20px] rounded-full bg-white border border-border flex items-center justify-center shrink-0">
          <svg :class="['w-2.5 h-2.5', getConfig(entry.action).color]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" :d="getConfig(entry.action).icon" />
          </svg>
        </div>

        <!-- Content -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-[12px] font-medium text-text">{{ entry.user }}</span>
            <span class="text-[12px] text-text-muted">{{ entry.action.toLowerCase() }}</span>
            <span class="text-[11px] text-text-light ml-auto shrink-0">{{ relativeTime(entry.timestamp) }}</span>
          </div>
          <!-- Changed fields with old → new values -->
          <div v-if="entry.action === 'Updated' && Array.isArray(entry.data?.changed) && (entry.data.changed as any[]).length > 0" class="mt-1 space-y-0.5">
            <div
              v-for="(change, ci) in (entry.data.changed as any[])"
              :key="ci"
              class="text-[11px] text-text-muted"
            >
              <span class="font-medium text-text">{{ change.field }}</span>:
              <span class="text-text-muted">{{ formatChangeValue(change.from) }}</span>
              <span class="text-text-light mx-0.5">&rarr;</span>
              <span class="text-emerald-600">{{ formatChangeValue(change.to) }}</span>
            </div>
          </div>
          <!-- Comment text -->
          <div v-if="entry.action === 'Commented' && entry.data?.comment" class="mt-1 px-3 py-2 bg-surface-muted rounded-lg text-[12px] text-text">
            {{ entry.data.comment }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
