import type { Episode, MediaDetail, MediaItem } from "./protocol";
import { normalizedTitle } from "./searchModel";

export interface SourceCandidate { pluginId: string; item: MediaItem }
export interface SourceEpisode { playlistId: string; episode: Episode }
export interface SourceStart { playlistId: string; episodeId: string; positionSec?: number }

export function metadataCompatible(a: MediaItem, b: MediaItem) {
  return (a.year === undefined || b.year === undefined || a.year === b.year)
    && (!a.type || !b.type || a.type === b.type);
}
export function sourceScore(reference: MediaItem, candidate: MediaItem) {
  const a = normalizedTitle(reference.title), b = normalizedTitle(candidate.title);
  if (!a || !b) return 0;
  const exact = a === b, compatible = metadataCompatible(reference, candidate);
  const grams = (value: string) => new Set(Array.from(value).map((c, i, chars) => c + (chars[i + 1] || "")));
  const left = grams(a), right = grams(b), common = [...left].filter(x => right.has(x)).length;
  const similarity = 2 * common / (left.size + right.size);
  return (exact && compatible ? 1000 : exact ? 500 : similarity * 100)
    + (compatible ? 10 : 0);
}
export function sourceEpisodes(detail: MediaDetail): SourceEpisode[] {
  return detail.playlists.flatMap(p => p.episodes.map(episode => ({ playlistId: p.id, episode })));
}
/** Incomplete pages cannot prove uniqueness across playlists. Index is never an identity. */
export function matchedSourceEpisode(previous: Episode | undefined, detail: MediaDetail): SourceEpisode | undefined {
  if (previous?.episodeNumber === undefined || detail.playlists.some(p => p.next)) return;
  const matches = sourceEpisodes(detail).filter(({ episode }) => episode.episodeNumber === previous.episodeNumber && episode.season === previous.season);
  return matches.length === 1 ? matches[0] : undefined;
}
