export const API_BASE_URL =
  import.meta.env.VITE_API_URL ?? "/api/v1";

export class ApiError extends Error {
  status: number;
  statusText: string;
  body: unknown;

  constructor(status: number, statusText: string, body: unknown) {
    super(`API ${status}: ${statusText}`);
    this.name = "ApiError";
    this.status = status;
    this.statusText = statusText;
    this.body = body;
  }
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let body: unknown;
    const fallback = response.clone();
    try {
      body = await response.json();
    } catch {
      body = await fallback.text();
    }
    throw new ApiError(response.status, response.statusText, body);
  }
  if (response.status === 204) {
    return undefined as T;
  }
  return response.json() as Promise<T>;
}

export async function apiGet<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    credentials: "include",
  });
  return handleResponse<T>(response);
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "include",
    body: JSON.stringify(body),
  });
  return handleResponse<T>(response);
}

export async function apiPut<T>(path: string, body: unknown): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    credentials: "include",
    body: JSON.stringify(body),
  });
  return handleResponse<T>(response);
}

export async function apiDelete<T = void>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    method: "DELETE",
    credentials: "include",
  });
  return handleResponse<T>(response);
}

export async function uploadAttachment(
  brewId: string,
  eventId: string,
  file: File,
): Promise<import("@/types").EventAttachmentResponse> {
  const form = new FormData();
  form.append("file", file);
  const response = await fetch(
    `${API_BASE_URL}/brews/${brewId}/events/${eventId}/attachments`,
    { method: "POST", credentials: "include", body: form },
  );
  return handleResponse(response);
}

export async function deleteAttachment(
  brewId: string,
  eventId: string,
  attachmentId: string,
): Promise<void> {
  return apiDelete(`/brews/${brewId}/events/${eventId}/attachments/${attachmentId}`);
}

/** Fetch a file from the API and hand it to the browser as a download. */
export async function downloadFile(path: string, filename: string): Promise<void> {
  const response = await fetch(`${API_BASE_URL}${path}`, { credentials: "include" });
  if (!response.ok) {
    throw new ApiError(response.status, response.statusText, null);
  }
  const url = URL.createObjectURL(await response.blob());
  try {
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
  } finally {
    URL.revokeObjectURL(url);
  }
}
