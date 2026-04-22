# Streamkeep

Streamkeep is an Android-first Tauri v2, Vue 3, TypeScript, and Rust application for assisted video capture.

The MVP workflow is:

1. open a streaming site inside the app
2. log in manually
3. start playback
4. detect a `master.m3u8` request from the Android WebView layer
5. confirm the detected stream
6. save the result locally on the phone as MP4
7. show progress and download history

## Development

Install dependencies:

```sh
pnpm install
```

Run the web frontend:

```sh
pnpm run dev:web
```

Run the Tauri desktop shell:

```sh
pnpm run dev
```

Run on a connected Android device:

```sh
pnpm run dev:android
```

Android is the primary target. Device tests must be performed on a real phone in debug mode.

