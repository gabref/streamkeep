import { defineStore } from 'pinia';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  listDownloadHistory,
  type DownloadJobRecord,
  type DownloadProgressPayload,
  type DownloadStatus,
} from '@/api/downloads';

export type DownloadJobStatus = DownloadStatus;

export type DownloadJob = {
  id: string;
  title: string;
  outputName: string;
  pageUrl: string;
  masterUrl: string;
  mediaPlaylistUrl?: string | null;
  referer?: string | null;
  userAgent?: string | null;
  cookies?: string | null;
  status: DownloadJobStatus;
  progress: number;
  quality: string;
  createdAt: string;
  updatedAt: string;
  errorMessage?: string | null;
  outputPath?: string | null;
  outputUri?: string | null;
  outputBytes?: number | null;
  thumbnailPath?: string | null;
  thumbnailUrl?: string | null;
};

export const useDownloadsStore = defineStore('downloads', {
  state: () => {
    return {
      jobs: [] as DownloadJob[],
      loaded: false,
      loadError: null as string | null,
    };
  },
  getters: {
    activeJobs(state): DownloadJob[] {
      return state.jobs.filter((job) => ['queued', 'preparing', 'downloading', 'remuxing'].includes(job.status));
    },
    completedJobs(state): DownloadJob[] {
      return state.jobs.filter((job) => job.status === 'done');
    },
    failedJobs(state): DownloadJob[] {
      return state.jobs.filter((job) => job.status === 'failed');
    },
    recentJobs(state): DownloadJob[] {
      return state.jobs.slice(0, 3);
    },
  },
  actions: {
    async loadHistory(): Promise<void> {
      try {
        this.jobs = (await listDownloadHistory()).map(recordToJob);
        this.loaded = true;
        this.loadError = null;
      } catch (error) {
        this.loadError = error instanceof Error ? error.message : String(error);
      }
    },
    upsertRecord(record: DownloadJobRecord): void {
      this.upsertJob(recordToJob(record));
    },
    replaceRecords(records: DownloadJobRecord[]): void {
      this.jobs = records.map(recordToJob);
      this.loaded = true;
      this.loadError = null;
      this.sortJobs();
    },
    applyProgress(progress: DownloadProgressPayload): void {
      const job = this.jobs.find((candidate) => candidate.id === progress.jobId);
      if (!job) {
        return;
      }

      const previousStatus = job.status;
      const nextProgress = progressPercent(progress);
      job.status = progress.status;
      job.progress = nextProgress === null ? job.progress : Math.max(job.progress, nextProgress);
      job.updatedAt = new Date().toISOString();
      if (progress.status === 'failed' && progress.message) {
        job.errorMessage = progress.message;
      }
      if (previousStatus !== job.status) {
        this.sortJobs();
      }
    },
    findJob(jobId: string): DownloadJob | undefined {
      return this.jobs.find((job) => job.id === jobId);
    },
    upsertJob(job: DownloadJob): void {
      const index = this.jobs.findIndex((candidate) => candidate.id === job.id);
      if (index === -1) {
        this.jobs.unshift(job);
      } else {
        this.jobs[index] = job;
      }
      this.sortJobs();
    },
    sortJobs(): void {
      this.jobs.sort((left, right) => Date.parse(right.updatedAt) - Date.parse(left.updatedAt));
    },
  },
});

function recordToJob(record: DownloadJobRecord): DownloadJob {
  return {
    id: record.id,
    title: record.title,
    outputName: record.outputName,
    pageUrl: record.pageUrl,
    masterUrl: record.masterUrl,
    mediaPlaylistUrl: record.mediaPlaylistUrl,
    referer: record.referer,
    userAgent: record.userAgent,
    cookies: record.cookies,
    status: record.status,
    progress: record.progress,
    quality: record.quality,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
    outputPath: record.outputPath,
    outputUri: record.outputUri,
    outputBytes: record.outputBytes,
    thumbnailPath: record.thumbnailPath,
    thumbnailUrl: record.thumbnailPath ? convertFileSrc(record.thumbnailPath) : null,
    errorMessage: record.errorMessage,
  };
}

function progressPercent(progress: DownloadProgressPayload): number | null {
  if (progress.totalBytes && progress.totalBytes > 0) {
    return Math.floor((Math.min(progress.downloadedBytes, progress.totalBytes) * 100) / progress.totalBytes);
  }
  if (progress.totalSegments && progress.totalSegments > 0) {
    const currentSegmentProgress =
      progress.currentSegmentDownloadedBytes && progress.currentSegmentTotalBytes
        ? Math.min(progress.currentSegmentDownloadedBytes, progress.currentSegmentTotalBytes) /
          progress.currentSegmentTotalBytes
        : 0;
    const completedSegments = Math.min(progress.completedSegments, progress.totalSegments);
    return Math.floor(((completedSegments + currentSegmentProgress) * 100) / progress.totalSegments);
  }
  return null;
}
