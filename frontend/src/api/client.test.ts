import { beforeEach, describe, expect, it, vi } from "vitest";
import { api, http, redirectUnauthorizedError, setUnauthorizedRedirectHandler } from "./client";

describe("api client", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    setUnauthorizedRedirectHandler(undefined);
  });

  it("propagates backend errors instead of returning sample data", async () => {
    const error = new Error("backend unavailable");
    vi.spyOn(http, "get").mockRejectedValue(error);

    await expect(api.tasks()).rejects.toBe(error);
  });

  it("redirects to the 401 page when backend returns unauthorized", async () => {
    const redirects: string[] = [];
    const error = { response: { status: 401 } };
    setUnauthorizedRedirectHandler((path) => redirects.push(path));

    await expect(redirectUnauthorizedError(error)).rejects.toBe(error);

    expect(redirects).toEqual(["/401"]);
  });
});
