import { defineStore } from 'pinia';

export type DownloadJobStatus =
  | 'queued'
  | 'preparing'
  | 'downloading'
  | 'remuxing'
  | 'done'
  | 'failed'
  | 'cancelled';

export type DownloadJob = {
  id: string;
  title: string;
  outputName: string;
  pageUrl: string;
  status: DownloadJobStatus;
  progress: number;
  quality: string;
  createdAt: string;
  updatedAt: string;
  errorMessage?: string;
  outputUri?: string;
};

const SAMPLE_JOBS: DownloadJob[] = [
  {
    id: 'sample-active',
    title: 'Detected stream',
    outputName: 'Detected stream.mp4',
    pageUrl: 'https://stream.example/watch',
    status: 'downloading',
    progress: 42,
    quality: 'Best available',
    createdAt: '2026-04-22T12:10:00Z',
    updatedAt: '2026-04-22T12:20:00Z',
  },
  {
    id: 'sample-complete',
    title: 'Saved capture',
    outputName: 'Saved capture.mp4',
    pageUrl: 'https://stream.example/library',
    status: 'done',
    progress: 100,
    quality: '1080p',
    createdAt: '2026-04-21T18:00:00Z',
    updatedAt: '2026-04-21T18:35:00Z',
    outputUri: 'content://downloads/streamkeep/Saved%20capture.mp4',
  },
  {
    id: 'sample-failed',
    title: 'Interrupted capture',
    outputName: 'Interrupted capture.mp4',
    pageUrl: 'https://stream.example/event',
    status: 'failed',
    progress: 12,
    quality: '720p',
    createdAt: '2026-04-20T20:00:00Z',
    updatedAt: '2026-04-20T20:05:00Z',
    errorMessage: 'Network request failed while downloading media segment 8.',
  },
];

export const useDownloadsStore = defineStore('downloads', {
  state: () => {
    return {
      jobs: SAMPLE_JOBS,
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
    findJob(jobId: string): DownloadJob | undefined {
      return this.jobs.find((job) => job.id === jobId);
    },
  },
});

