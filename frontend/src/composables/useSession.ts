import { ref } from "vue";

const user = ref("Guest");
const roles = ref<string[]>([]);
const loaded = ref(false);
const authenticated = ref(false);

async function load() {
  try {
    const res = await fetch("/api/session", { credentials: "include" });
    if (res.ok) {
      const data = await res.json();
      user.value = data.user;
      roles.value = data.roles || [];
      authenticated.value = true;
    } else {
      reset();
    }
  } catch {
    reset();
  }
  loaded.value = true;
}

function reset() {
  user.value = "Guest";
  roles.value = [];
  authenticated.value = false;
}

async function logout() {
  try {
    await fetch("/api/auth/logout", { method: "POST", credentials: "include" });
  } catch {
    // ignore
  }
  reset();
}

export function useSession() {
  const isAdmin = () =>
    user.value === "Administrator" || roles.value.includes("Administrator");
  return { user, roles, loaded, authenticated, isAdmin, logout, load };
}
