import { http, HttpResponse } from "msw"

export const API = "/api/v1"

export const handlers = [
  http.get(`${API}/auth/me`, () =>
    HttpResponse.json({ userSub: "u1", email: "dev@example.com", name: "Dev" }),
  ),
  http.post(`${API}/auth/logout`, () => new HttpResponse(null, { status: 204 })),
]
