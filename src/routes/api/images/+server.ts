import { json, type RequestHandler } from '@sveltejs/kit';
import { Datastore } from '@google-cloud/datastore';

const projectId =
	process.env.GOOGLE_CLOUD_PROJECT || process.env.GCLOUD_PROJECT || 'taigaishida-217622';
const datastore = new Datastore({ projectId });

type Entity = Record<string, unknown>;

export const GET: RequestHandler = async ({ url }) => {
	try {
		const limit = Math.min(Number(url.searchParams.get('limit')) || 20, 100);
		const cursor = url.searchParams.get('cursor') || undefined;

		let query = datastore.createQuery('Image').order('created', { descending: true }).limit(limit);
		if (cursor) query = query.start(cursor);

		const [entities, info] = (await datastore.runQuery(query)) as [
			Entity[],
			Record<string, unknown>
		];

		const items = (entities || []).map((e) => {
			const haiku: string[] = Array.isArray(e.haiku) ? e.haiku : [];
			return {
				url: e.public_url as string,
				haiku,
				line1: haiku[0] ?? '',
				line2: haiku[1] ?? '',
				line3: haiku[2] ?? '',
				brightness: (e.brightness as number) ?? 0,
				created: (e.created as string) ?? null
			};
		});

		const moreResults = info?.moreResults;
		const nextCursor = moreResults && moreResults !== 'NO_MORE_RESULTS' ? info?.endCursor : null;

		return json({ items, cursor: nextCursor });
	} catch (err) {
		console.error('Failed to load images from Datastore', err);
		return json({ items: [], cursor: null, error: 'Failed to load images' }, { status: 500 });
	}
};
