import type { PageServerLoad } from "./$types";
import { Datastore } from "@google-cloud/datastore";

const projectId =
  process.env.GOOGLE_CLOUD_PROJECT ||
  process.env.GCLOUD_PROJECT ||
  "taigaishida-217622";
const datastore = new Datastore({ projectId });

export const load: PageServerLoad = async ({ setHeaders, url }) => {
  try {
    const limit = Math.min(Number(url.searchParams.get("limit")) || 20, 100);
    const query = datastore
      .createQuery("Image")
      .order("created", { descending: true })
      .limit(limit);
    const [entities, info] = await datastore.runQuery(query);

    const items = (entities || []).map((e: Record<string, unknown>) => {
      const haiku: string[] = Array.isArray(e.haiku) ? e.haiku : [];
      return {
        url: e.public_url as string,
        haiku,
        line1: haiku[0] ?? "",
        line2: haiku[1] ?? "",
        line3: haiku[2] ?? "",
        brightness: (e.brightness as number) ?? 0,
        created: (e.created as string) ?? null,
      };
    });

    const moreResults = (info as Record<string, unknown>)?.moreResults;
    const cursor =
      moreResults && moreResults !== "NO_MORE_RESULTS"
        ? (info as Record<string, unknown>)?.endCursor
        : null;

    // Light caching for faster subsequent requests (tune as needed)
    setHeaders({ "Cache-Control": "public, max-age=60" });

    return { items, cursor, limit };
  } catch (err) {
    console.error("Failed to load images from Datastore", err);
    return { items: [], cursor: null, limit: 20 };
  }
};
