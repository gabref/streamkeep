# Streamkeep Android Testing

This project is Android-first. Automated checks validate the Rust and Vue layers, but real-device testing is required for WebView interception, playback, remuxing, MediaStore publishing, and Android intents.

## Automated Checks

Run before committing:

```sh
pnpm run format:check
pnpm run lint:rust
pnpm run type-check
pnpm run eslint
pnpm run test:unit
pnpm run test:rust
pnpm run build:android:apk
```

## Device Smoke Test

Use a phone connected in debug mode.

```sh
adb devices
pnpm run build:android:apk
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
adb shell monkey -p app.streamkeep.mobile 1
```

On the phone:

1. Open Player.
2. Enter a page or HLS test URL.
3. Start playback.
4. Confirm the detection dialog.
5. Start the download.
6. Wait for completion.
7. Open Downloads and verify the job is done.
8. Open the detail screen and tap Open.

Expected results:

- the detection dialog appears after `.m3u8` playback requests
- progress moves through preparing, downloading, remuxing, and done
- history persists after app restart
- the MP4 exists under `Download/Streamkeep`
- the Open action launches Android's file chooser/player

Useful inspection commands:

```sh
adb shell run-as app.streamkeep.mobile cat downloads/history.json
adb shell run-as app.streamkeep.mobile ls -l downloads
adb shell ls -l /sdcard/Download/Streamkeep
adb logcat -d -t 500 | grep -i streamkeep
```
