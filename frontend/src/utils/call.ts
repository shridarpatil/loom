interface CallOptions {
  method?: "GET" | "POST" | "PUT" | "DELETE";
  body?: unknown;
  params?: Record<string, string>;
}

interface ApiError {
  error: string;
  error_type: string;
}

export class LoomApiError extends Error {
  error_type: string;
  status: number;

  constructor(message: string, error_type: string, status: number) {
    super(message);
    this.error_type = error_type;
    this.status = status;
  }
}

async function request<T>(url: string, options: CallOptions = {}): Promise<T> {
  const { method = "GET", body, params } = options;

  let fullUrl = url;
  if (params) {
    const qs = new URLSearchParams(params).toString();
    fullUrl += `?${qs}`;
  }

  const headers: Record<string, string> = {};
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
  }

  const res = await fetch(fullUrl, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
    credentials: "include",
  });

  if (!res.ok) {
    let err: ApiError = { error: res.statusText, error_type: "ServerError" };
    try {
      err = await res.json();
    } catch {
      // use default
    }
    throw new LoomApiError(err.error, err.error_type, res.status);
  }

  if (res.status === 204) return undefined as T;
  return res.json();
}

export const loom = {
  call<T = unknown>(method: string, args: Record<string, unknown> = {}): Promise<T> {
    return request<T>(`/api/method/${method}`, {
      method: "POST",
      body: args,
    });
  },

  resource(doctype: string) {
    const base = `/api/resource/${encodeURIComponent(doctype)}`;

    return {
      getList(options: {
        filters?: unknown[] | Record<string, unknown>;
        fields?: string[];
        order_by?: string;
        limit?: number;
        offset?: number;
        search_term?: string;
      } = {}): Promise<{ data: Record<string, unknown>[] }> {
        const params: Record<string, string> = {};
        if (options.filters) params.filters = JSON.stringify(options.filters);
        if (options.fields) params.fields = JSON.stringify(options.fields);
        if (options.order_by) params.order_by = options.order_by;
        if (options.limit != null) params.limit = String(options.limit);
        if (options.offset != null) params.offset = String(options.offset);
        if (options.search_term) params.search_term = options.search_term;
        return request(base, { params });
      },

      get(id: string): Promise<{ data: Record<string, unknown> }> {
        return request(`${base}/${encodeURIComponent(id)}`);
      },

      insert(doc: Record<string, unknown>): Promise<{ data: Record<string, unknown> }> {
        return request(base, { method: "POST", body: doc });
      },

      update(
        id: string,
        doc: Record<string, unknown>,
      ): Promise<{ data: Record<string, unknown> }> {
        return request(`${base}/${encodeURIComponent(id)}`, {
          method: "PUT",
          body: doc,
        });
      },

      delete(id: string): Promise<void> {
        return request(`${base}/${encodeURIComponent(id)}`, {
          method: "DELETE",
        });
      },

      submit(id: string): Promise<{ data: Record<string, unknown> }> {
        return request(`${base}/${encodeURIComponent(id)}/submit`, {
          method: "POST",
        });
      },

      cancel(id: string): Promise<{ data: Record<string, unknown> }> {
        return request(`${base}/${encodeURIComponent(id)}/cancel`, {
          method: "POST",
        });
      },
    };
  },

  async getMeta(
    doctype: string,
  ): Promise<{ data: DocTypeMeta }> {
    return request(`/api/doctype/${encodeURIComponent(doctype)}`);
  },

  activity(doctype: string, name: string) {
    const base = `/api/activity/${encodeURIComponent(doctype)}/${encodeURIComponent(name)}`;
    return {
      get(limit = 50): Promise<{ data: ActivityEntry[] }> {
        return request(base, { params: { limit: String(limit) } });
      },
      comment(content: string): Promise<{ message: string }> {
        return request(`${base}/comment`, { method: "POST", body: { content } });
      },
    };
  },
};

export interface DocTypeMeta {
  name: string;
  module: string;
  naming_rule: string;
  autoname?: string;
  is_submittable: boolean;
  is_child_table: boolean;
  is_single: boolean;
  title_field?: string;
  search_fields?: string[];
  sort_field?: string;
  sort_order?: string;
  fields: DocFieldMeta[];
  permissions: DocPermMeta[];
  client_script?: string;
}

export interface DocFieldMeta {
  fieldname: string;
  label?: string;
  fieldtype: string;
  options?: string;
  reqd?: boolean;
  unique?: boolean;
  read_only?: boolean;
  hidden?: boolean;
  set_only_once?: boolean;
  default?: string;
  in_list_view?: boolean;
  in_standard_filter?: boolean;
  fetch_from?: string;
  depends_on?: string;
  mandatory_depends_on?: string;
  read_only_depends_on?: string;
  collapsible?: boolean;
  description?: string;
  permlevel?: number;
  length?: number;
  precision?: number;
}

export interface ActivityEntry {
  id: number;
  doctype: string;
  docname: string;
  action: string;
  user: string;
  timestamp: string;
  data: Record<string, unknown>;
}

export interface DocPermMeta {
  role: string;
  permlevel: number;
  read: boolean;
  write: boolean;
  create: boolean;
  delete: boolean;
  submit: boolean;
  cancel: boolean;
  if_owner?: boolean;
}
