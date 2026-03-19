const URL_SAFE_SLUG = /^[a-z0-9][a-z0-9-]*[a-z0-9]$/;
const SINGLE_CHAR_SLUG = /^[a-z0-9]$/;

/** Throw if slug is not URL-safe (lowercase alphanumeric + hyphens, no leading/trailing hyphens). */
export function assertUrlSafeSlug(slug: string): void {
  if (slug.length === 0) {
    throw new Error("slug cannot be empty");
  }
  if (slug.length === 1) {
    if (!SINGLE_CHAR_SLUG.test(slug)) {
      throw new Error(`slug is not URL-safe: '${slug}' (only a-z, 0-9, hyphens allowed)`);
    }
    return;
  }
  if (!URL_SAFE_SLUG.test(slug)) {
    throw new Error(`slug is not URL-safe: '${slug}' (only a-z, 0-9, hyphens allowed, no leading/trailing hyphens)`);
  }
}
