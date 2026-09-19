import { invoke } from "@tauri-apps/api/core";

export interface FeedPage {
  feeds: Record<string, unknown>[];
  attachInfo?: string;
  hasMore: boolean;
}

export function fetchFirstFeeds() {
  return invoke<FeedPage>("fetch_first_feeds");
}

export function fetchMoreFeeds(attachInfo: string) {
  return invoke<FeedPage>("fetch_more_feeds", { attachInfo });
}

export type ArchiveStatus = "idle" | "running" | "completed" | "cancelled" | "limited" | "error";
export interface ArchiveProgress { status: ArchiveStatus; pages: number; fetched: number; saved: number; skipped: number; message: string; retryAt?: number; }
export interface ArchiveSkipItem {
  id: number; pageNumber: number; cursorOffset: number; offsetAdvance: number; baseTime: number;
  error: string; skippedAt: number; retryCount: number; lastRetryAt?: number; resolvedAt?: number; recoveredRecords: number;
}
export interface ArchiveSkipRetryResult { success: boolean; message: string; recoveredRecords: number; }
export interface ArchiveItem {
  id: number; cellId: string; publishedAt: number; content?: string; authorUin?: string;
  authorName?: string; pictureUrls: string[]; videoUrl?: string; videoUrls: string[]; videoCoverUrl?: string; likeCount: number; commentCount: number;
  likes: LikeUser[];
  comments: ArchiveComment[];
}
export interface LikeUser { uin?: string; nickname?: string; historical: boolean; likedAt: number; }
export interface ArchiveReply { uin?: string; nickname?: string; replyToUin?: string; replyToNickname?: string; content: string; createdAt: number; }
export interface ArchiveComment { uin?: string; nickname?: string; content: string; createdAt: number; replies: ArchiveReply[]; }
export type ArchiveCategory = "self" | "other" | "guestbook";
export interface ArchiveMediaItem { key: string; dynamicId: number; mediaType: "photo" | "video"; pictureIndex?: number; url: string; coverUrl?: string; publishedAt: number; authorUin?: string; authorName?: string; content?: string; }
export interface ArchiveMediaPage { items: ArchiveMediaItem[]; total: number; years: number[]; }
export const startFeedArchive = (intervalMs: number) => invoke<ArchiveProgress>("start_feed_archive", { intervalMs });
export const getArchiveProgress = () => invoke<ArchiveProgress>("get_archive_progress");
export const cancelFeedArchive = () => invoke<void>("cancel_feed_archive");
export const listArchiveSkips = () => invoke<ArchiveSkipItem[]>("list_archive_skips");
export const retryArchiveSkip = (id: number) => invoke<ArchiveSkipRetryResult>("retry_archive_skip", { id });
export const listArchivedFeeds = (limit = 100, offset = 0, category: ArchiveCategory = "self", year?: number, descending = true, query?: string) => invoke<ArchiveItem[]>("list_archived_feeds", { limit, offset, category, year, descending, query });
export const listArchiveYears = (category: ArchiveCategory = "self") => invoke<number[]>("list_archive_years", { category });
export const listArchivedMedia = (limit = 60, offset = 0, year?: number) => invoke<ArchiveMediaPage>("list_archived_media", { limit, offset, year });
export const getArchivedFeed = (id: number) => invoke<ArchiveItem>("get_archived_feed", { id });
export const countArchivedFeeds = (category: ArchiveCategory = "self", year?: number, query?: string) => invoke<number>("count_archived_feeds", { category, year, query });
export const exportArchivedHtml = (category: ArchiveCategory, ids?: number[]) => invoke<string>("export_archived_html", { category, ids });
export const loadArchivedImage = (id: number, pictureIndex: number) => invoke<string>("load_archived_image", { id, pictureIndex });
export const loadArchivedVideo = (id: number) => invoke<string>("load_archived_video", { id });
export interface ArchiveOverview { dynamics: number; pictures: number; comments: number; likes: number; databaseBytes: number; }
export const getArchiveOverview = () => invoke<ArchiveOverview>("get_archive_overview");
export interface Interactor { uin: string; nickname: string; likes: number; comments: number; total: number; lastAt: number; }
export const listInteractors = () => invoke<Interactor[]>("list_interactors");
export const listContactCommentThreads = (uin: string) => invoke<ArchiveItem[]>("list_contact_comment_threads", { uin });
export interface InteractionRank { uin: string; nickname: string; interactions: number; likes: number; comments: number; }
export const getInteractionRanking = (limit = 8) => invoke<InteractionRank[]>("get_interaction_ranking", { limit });
export const deleteArchivedFeeds = (ids: number[]) => invoke<number>("delete_archived_feeds", { ids });
export const clearArchivedFeeds = () => invoke<number>("clear_archived_feeds");
export const deleteAllAppData = () => invoke<void>("delete_all_app_data");

export const openRecyclePasswordWindow = () => invoke<void>("open_recycle_password_window");
export const prepareRecyclePasswordWindow = () => invoke<string>("prepare_recycle_password_window");
export const checkRecyclePassword = () => invoke<string | null>("check_recycle_password");
export const closeRecyclePasswordWindow = () => invoke<void>("close_recycle_password_window");
export const listRecycleAlbums = (pwd2sig: string) => invoke<Record<string, unknown>>("list_recycle_albums", { pwd2sig });
export const listRecyclePhotos = (pwd2sig: string, albumId?: string) => invoke<Record<string, unknown>>("list_recycle_photos", { pwd2sig, albumId });
export const listQzoneAlbums = () => invoke<Record<string, unknown>>("list_qzone_albums");
export const createQzoneAlbum = (name: string) => invoke<Record<string, unknown>>("create_qzone_album", { name });
export const recoverRecycleAlbum = (pwd2sig: string, albumId: string) => invoke<Record<string, unknown>>("recover_recycle_album", { pwd2sig, albumId });
export const recoverRecyclePhotos = (pwd2sig: string, sourceAlbumId: string, targetAlbumId: string, photoIds: string[]) =>
  invoke<Record<string, unknown>>("recover_recycle_photos", { pwd2sig, sourceAlbumId, targetAlbumId, photoIds });
export const loadRecyclePhotoPreview = (imageUrl: string) => invoke<string>("load_recycle_photo_preview", { imageUrl });

export type LibraryModule = "albums" | "photos" | "videos" | "guestbook" | "favorites";
export interface LibraryItem {
  id: number; module: LibraryModule; itemKey: string; parentKey: string; createdAt: number;
  title: string; summary: string; authorUin?: string; authorName?: string; coverUrl?: string; mediaUrls: string[];
}
export interface LibraryPage {
  items: LibraryItem[]; total: number; remoteTotal: number; complete: boolean; syncedAt: number; lastError?: string;
}
export interface LibrarySyncResult {
  module: LibraryModule; fetched: number; saved: number; remoteTotal: number; complete: boolean; message: string;
}
export const syncQzoneLibrary = (module: LibraryModule, parentKey?: string) =>
  invoke<LibrarySyncResult>("sync_qzone_library", { module, parentKey });
export const listQzoneLibrary = (module: LibraryModule, parentKey?: string, query?: string, year?: number, limit = 60, offset = 0) =>
  invoke<LibraryPage>("list_qzone_library", { module, parentKey, query, year, limit, offset });
export const listQzoneLibraryYears = (module: LibraryModule, parentKey?: string) =>
  invoke<number[]>("list_qzone_library_years", { module, parentKey });
