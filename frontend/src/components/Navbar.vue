<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useSession } from "@/composables/useSession";

const router = useRouter();
const { user, logout } = useSession();
const searchQuery = ref("");
const showUserMenu = ref(false);

function handleSearch() {
  if (searchQuery.value.trim()) {
    router.push(`/app/${searchQuery.value.trim()}`);
    searchQuery.value = "";
  }
}

function handleLogout() {
  showUserMenu.value = false;
  logout();
  router.push("/login");
}

function goHome() {
  router.push("/app");
}
</script>

<template>
  <nav class="shrink-0 bg-white border-b border-gray-200/70 z-30">
    <div class="relative flex items-center px-5 h-12">
      <!-- Left: Logo + page header (z-10 to stay above centered search) -->
      <div class="flex items-center gap-3 min-w-0 shrink-0 relative z-10">
        <button class="flex items-center gap-2.5 hover:opacity-80 transition-opacity shrink-0" @click="goHome">
          <div class="w-7 h-7 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center shadow-sm">
            <svg class="w-4 h-4 text-white" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3 6.75A3.75 3.75 0 0 1 6.75 3h10.5A3.75 3.75 0 0 1 21 6.75v10.5A3.75 3.75 0 0 1 17.25 21H6.75A3.75 3.75 0 0 1 3 17.25V6.75ZM7.5 8.25a.75.75 0 0 0 0 1.5h4.5a.75.75 0 0 0 0-1.5h-4.5Zm0 3a.75.75 0 0 0 0 1.5h9a.75.75 0 0 0 0-1.5h-9Zm0 3a.75.75 0 0 0 0 1.5h6a.75.75 0 0 0 0-1.5h-6Z" />
            </svg>
          </div>
        </button>
        <div id="navbar-page-header" class="flex items-center gap-3 min-w-0" />
      </div>

      <!-- Center: Search — absolutely positioned so it stays centered regardless of left/right width -->
      <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
        <div class="flex items-center gap-2 bg-gray-100/80 rounded-lg px-3 py-1.5 w-[300px]">
          <svg class="w-3.5 h-3.5 text-gray-400 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
          </svg>
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Search or type a command (Ctrl+K)"
            class="bg-transparent border-none outline-none text-gray-700 text-[13px] w-full placeholder:text-gray-400"
            @keyup.enter="handleSearch"
          />
        </div>
      </div>

      <!-- Right: Page actions + global actions (z-10 to stay above centered search) -->
      <div class="flex items-center gap-2 ml-auto shrink-0 relative z-10">
        <div id="navbar-page-actions" class="flex items-center gap-2" />
        <div class="w-px h-5 bg-gray-200 mx-1" />
        <button class="w-8 h-8 rounded-lg flex items-center justify-center hover:bg-gray-100 transition-colors">
          <svg class="w-[18px] h-[18px] text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M14.857 17.082a23.848 23.848 0 0 0 5.454-1.31A8.967 8.967 0 0 1 18 9.75V9A6 6 0 0 0 6 9v.75a8.967 8.967 0 0 1-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 0 1-5.714 0m5.714 0a3 3 0 1 1-5.714 0" />
          </svg>
        </button>

        <span class="text-[13px] text-gray-500 cursor-pointer hover:text-gray-700 transition-colors select-none px-1">Help</span>

        <div class="relative">
          <button
            class="w-8 h-8 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 text-white text-[12px] font-semibold flex items-center justify-center shadow-sm hover:shadow-md transition-shadow"
            @click="showUserMenu = !showUserMenu"
          >
            {{ (user || 'A').charAt(0).toUpperCase() }}
          </button>
          <div
            v-if="showUserMenu"
            class="absolute right-0 top-full mt-1.5 w-44 bg-white rounded-xl shadow-lg border border-gray-200/60 py-1.5 z-50"
          >
            <div class="px-3.5 py-2 text-[12px] text-gray-500 border-b border-gray-100">{{ user }}</div>
            <button
              class="w-full text-left px-3.5 py-2 text-[13px] text-gray-700 hover:bg-gray-50 transition-colors rounded-b-xl"
              @click="handleLogout"
            >
              Log out
            </button>
          </div>
        </div>
      </div>
    </div>
  </nav>
</template>
