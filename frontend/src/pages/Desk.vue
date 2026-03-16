<script setup lang="ts">
import { onMounted } from "vue";
import Sidebar from "@/components/Sidebar.vue";
import LToastContainer from "@/components/ui/LToastContainer.vue";
import { socket } from "@/utils/socket";

onMounted(() => socket.connect());
</script>

<template>
  <div class="flex h-screen overflow-hidden bg-surface-muted">
    <Sidebar />
    <main class="flex-1 overflow-y-auto">
      <RouterView v-slot="{ Component }">
        <div :key="$route.fullPath" class="animate-fade-in">
          <component :is="Component" />
        </div>
      </RouterView>
    </main>
    <LToastContainer />
  </div>
</template>
