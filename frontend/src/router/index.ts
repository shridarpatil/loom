import { createRouter, createWebHistory } from "vue-router";
import { useSession } from "@/composables/useSession";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/app",
    },
    {
      path: "/login",
      name: "login",
      component: () => import("@/pages/Login.vue"),
      meta: { public: true },
    },
    {
      path: "/app",
      name: "desk",
      component: () => import("@/pages/Desk.vue"),
      children: [
        {
          path: "",
          name: "home",
          component: () => import("@/pages/Workspace.vue"),
        },
        {
          // DocType list
          path: "DocType",
          name: "doctype-list",
          component: () => import("@/pages/ListView.vue"),
          props: { doctype: "DocType" },
        },
        {
          // New DocType → DocType Builder
          path: "DocType/new",
          name: "doctype-new",
          component: () => import("@/pages/DocTypeBuilder.vue"),
        },
        {
          // Edit DocType → DocType Builder
          path: "DocType/:doctype",
          name: "doctype-edit",
          component: () => import("@/pages/DocTypeBuilder.vue"),
          props: true,
        },
        {
          path: ":doctype",
          name: "list",
          component: () => import("@/pages/RouteResolver.vue"),
          props: true,
        },
        {
          path: ":doctype/new",
          name: "new-doc",
          component: () => import("@/pages/Form.vue"),
          props: (route) => ({ doctype: route.params.doctype, id: null }),
        },
        {
          path: ":doctype/:id",
          name: "form",
          component: () => import("@/pages/Form.vue"),
          props: true,
        },
        {
          path: "customize-form/:doctype",
          name: "customize-form",
          component: () => import("@/pages/CustomizeForm.vue"),
          props: true,
        },
        {
          path: "report-builder",
          name: "report-builder",
          component: () => import("@/pages/Report.vue"),
        },
        {
          path: "role-permission-manager",
          name: "role-permission-manager",
          component: () => import("@/pages/RolePermissionManager.vue"),
        },
        {
          path: "role-permission-manager/:doctype",
          name: "role-permission-manager-doctype",
          component: () => import("@/pages/RolePermissionManager.vue"),
          props: true,
        },
      ],
    },
  ],
});

// Navigation guard — check session on every protected navigation
router.beforeEach(async (to) => {
  if (to.meta.public) return true;

  const { loaded, authenticated, load } = useSession();

  if (!loaded.value) {
    await load();
  }

  if (!authenticated.value) {
    return { path: "/login", query: { redirect: to.fullPath } };
  }

  return true;
});

export default router;
