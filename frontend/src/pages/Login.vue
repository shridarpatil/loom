<script setup lang="ts">
import { ref } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useSession } from "@/composables/useSession";
import { useTheme } from "@/composables/useTheme";
import { LButton, LInput, LAlert } from "@/components/ui";

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
  <div class="min-h-screen bg-surface-muted flex items-center justify-center px-4">
    <div class="w-full max-w-sm">
      <!-- Logo -->
      <div class="text-center mb-8">
        <img v-if="theme.logo_url" :src="theme.logo_url" :alt="theme.brand_name" class="h-8 mx-auto mb-2" />
        <h1 class="text-xl font-bold text-primary-600 tracking-tight">{{ theme.brand_name }}</h1>
        <p class="text-[13px] text-text-muted mt-1">Sign in to your account</p>
      </div>

      <!-- Form -->
      <div class="bg-white border border-border rounded-lg px-6 py-5">
        <LAlert v-if="error" type="error" dismissible class="mb-4" @dismiss="error = ''">
          {{ error }}
        </LAlert>

        <form @submit.prevent="login" class="space-y-4">
          <div>
            <label class="block text-[12px] font-medium text-text-muted mb-1">Email</label>
            <LInput
              v-model="email"
              type="text"
              placeholder="you@example.com"
              class="h-9"
            />
          </div>
          <div>
            <label class="block text-[12px] font-medium text-text-muted mb-1">Password</label>
            <LInput
              v-model="password"
              type="password"
              placeholder="Password"
              class="h-9"
            />
          </div>
          <LButton
            type="submit"
            :disabled="!email || !password"
            :loading="loading"
            class="w-full h-9 justify-center"
          >
            {{ loading ? "Signing in..." : "Sign in" }}
          </LButton>
        </form>
      </div>

      <p class="text-center text-[11px] text-text-light mt-4">
        Default: Administrator / admin
      </p>
    </div>
  </div>
</template>
