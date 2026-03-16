<script setup lang="ts">
import { ref } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useSession } from "@/composables/useSession";
import { useTheme } from "@/composables/useTheme";
import { LButton, LAlert } from "@/components/ui";

const router = useRouter();
const route = useRoute();
const { load: reloadSession } = useSession();
const { theme } = useTheme();
const email = ref("Administrator");
const password = ref("");
const error = ref("");
const loading = ref(false);

async function login() {
  loading.value = true;
  error.value = "";
  try {
    const res = await fetch("/api/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email: email.value, password: password.value }),
      credentials: "include",
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({ error: "Login failed" }));
      error.value = data.error || "Login failed";
      return;
    }
    await reloadSession();
    const redirect = (route.query.redirect as string) || "/app";
    router.replace(redirect);
  } catch {
    error.value = "Network error";
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen bg-gradient-to-br from-slate-50 via-white to-primary-50 flex items-center justify-center px-4">
    <!-- Decorative background -->
    <div class="absolute inset-0 overflow-hidden pointer-events-none">
      <div class="absolute -top-40 -right-40 w-80 h-80 bg-primary-100/40 rounded-full blur-3xl" />
      <div class="absolute -bottom-40 -left-40 w-80 h-80 bg-primary-50/60 rounded-full blur-3xl" />
    </div>

    <div class="w-full max-w-[380px] relative animate-slide-up">
      <!-- Logo & Branding -->
      <div class="text-center mb-8">
        <div class="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-primary-600 shadow-lg shadow-primary-600/20 mb-4">
          <img v-if="theme.logo_url" :src="theme.logo_url" :alt="theme.brand_name" class="h-8" />
          <svg v-else class="w-7 h-7 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6.429 9.75 2.25 12l4.179 2.25m0-4.5 5.571 3 5.571-3m-11.142 0L2.25 7.5 12 2.25l9.75 5.25-4.179 2.25m0 0L21.75 12l-4.179 2.25m0 0L12 17.25 6.429 14.25m11.142 0 4.179 2.25L12 21.75l-9.75-5.25 4.179-2.25" />
          </svg>
        </div>
        <h1 class="text-2xl font-bold text-text tracking-tight">{{ theme.brand_name }}</h1>
        <p class="text-[13px] text-text-muted mt-1">Sign in to continue</p>
      </div>

      <!-- Form Card -->
      <div class="bg-white border border-border/60 rounded-2xl shadow-xl shadow-black/[0.03] px-7 py-6">
        <LAlert v-if="error" type="error" dismissible class="mb-5" @dismiss="error = ''">
          {{ error }}
        </LAlert>

        <form @submit.prevent="login" class="space-y-4">
          <div>
            <label class="block text-[12px] font-medium text-text mb-1.5">Email</label>
            <input
              v-model="email"
              type="text"
              placeholder="you@example.com"
              class="w-full h-10 px-3 text-[13px] border border-border rounded-lg bg-white text-text placeholder-text-light focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-all"
            />
          </div>
          <div>
            <label class="block text-[12px] font-medium text-text mb-1.5">Password</label>
            <input
              v-model="password"
              type="password"
              placeholder="Password"
              class="w-full h-10 px-3 text-[13px] border border-border rounded-lg bg-white text-text placeholder-text-light focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-all"
            />
          </div>
          <LButton
            type="submit"
            :disabled="!email || !password"
            :loading="loading"
            class="w-full h-10 justify-center text-[14px]"
          >
            {{ loading ? "Signing in..." : "Sign in" }}
          </LButton>
        </form>
      </div>

      <p class="text-center text-[11px] text-text-light mt-5">
        Default credentials: <span class="text-text-muted font-medium">Administrator</span> / <span class="text-text-muted font-medium">admin</span>
      </p>
    </div>
  </div>
</template>
