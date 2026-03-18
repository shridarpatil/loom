<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import Navbar from "@/components/Navbar.vue";
import Sidebar from "@/components/Sidebar.vue";
import LToastContainer from "@/components/ui/LToastContainer.vue";
import { socket } from "@/utils/socket";

const route = useRoute();
const isHome = computed(() => route.path === "/app" || route.path === "/app/");

onMounted(() => socket.connect());
</script>

<template>
  <div class="flex flex-col h-screen overflow-hidden">
    <Navbar />
    <div class="flex flex-1 overflow-hidden bg-surface-muted">
      <Sidebar v-if="!isHome" />
      <main class="flex-1 overflow-y-auto">
        <RouterView v-slot="{ Component }">
          <div :key="$route.fullPath" class="animate-fade-in h-full">
            <component :is="Component" />
          </div>
        </RouterView>
      </main>
    </div>
    <LToastContainer />
  </div>
</template>
