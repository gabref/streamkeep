package app.streamkeep.capture

import android.app.Activity
import android.content.Intent
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class OpenPlayerArgs {
  var url: String? = null
}

@InvokeArg
class LoadUrlArgs {
  lateinit var url: String
}

@InvokeArg
class RemuxToMp4Args {
  lateinit var inputPath: String
  lateinit var outputPath: String
}

@InvokeArg
class PublishToDownloadsArgs {
  lateinit var inputPath: String
  lateinit var displayName: String
}

@TauriPlugin
class StreamkeepCapturePlugin(private val activity: Activity) : Plugin(activity) {
  init {
    StreamkeepPlayerRegistry.attachPlugin(this)
  }

  @Command
  fun register_listener(invoke: Invoke) {
    registerListener(invoke)
  }

  @Command
  fun remove_listener(invoke: Invoke) {
    removeListener(invoke)
  }

  @Command
  fun openPlayer(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(OpenPlayerArgs::class.java)
      val intent = Intent(activity, StreamkeepPlayerActivity::class.java)
      intent.putExtra(StreamkeepPlayerActivity.EXTRA_INITIAL_URL, normalizeUrl(args.url))
      activity.startActivity(intent)
      invoke.resolve(StreamkeepPlayerRegistry.snapshot())
    } catch (ex: Exception) {
      invoke.reject(ex.message ?: "Failed to open Streamkeep player")
    }
  }

  @Command
  fun getPlayerState(invoke: Invoke) {
    invoke.resolve(StreamkeepPlayerRegistry.snapshot())
  }

  @Command
  fun goBack(invoke: Invoke) {
    StreamkeepPlayerRegistry.runOnPlayer(invoke) { player ->
      player.goBack()
    }
  }

  @Command
  fun goForward(invoke: Invoke) {
    StreamkeepPlayerRegistry.runOnPlayer(invoke) { player ->
      player.goForward()
    }
  }

  @Command
  fun reload(invoke: Invoke) {
    StreamkeepPlayerRegistry.runOnPlayer(invoke) { player ->
      player.reload()
    }
  }

  @Command
  fun loadUrl(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(LoadUrlArgs::class.java)
      StreamkeepPlayerRegistry.runOnPlayer(invoke) { player ->
        player.loadUrl(normalizeUrl(args.url))
      }
    } catch (ex: Exception) {
      invoke.reject(ex.message ?: "Failed to load URL")
    }
  }

  @Command
  fun remuxToMp4(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(RemuxToMp4Args::class.java)
      val result = StreamkeepMp4Remuxer.remuxToMp4(args.inputPath, args.outputPath)
      val payload = JSObject()
      payload.put("outputPath", result.outputPath)
      payload.put("trackCount", result.trackCount)
      payload.put("outputBytes", result.outputBytes)
      invoke.resolve(payload)
    } catch (ex: Exception) {
      invoke.reject(ex.message ?: "Failed to remux media to MP4")
    }
  }

  @Command
  fun publishToDownloads(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(PublishToDownloadsArgs::class.java)
      val result = StreamkeepMediaStorePublisher.publishToDownloads(
        activity = activity,
        inputPath = args.inputPath,
        displayName = args.displayName
      )
      val payload = JSObject()
      payload.put("contentUri", result.contentUri)
      payload.put("displayName", result.displayName)
      payload.put("relativePath", result.relativePath)
      payload.put("outputBytes", result.outputBytes)
      invoke.resolve(payload)
    } catch (ex: Exception) {
      invoke.reject(ex.message ?: "Failed to publish MP4 to Downloads")
    }
  }

  private fun normalizeUrl(value: String?): String {
    val trimmed = value?.trim().orEmpty()
    if (trimmed.isEmpty()) {
      return StreamkeepPlayerActivity.DEFAULT_URL
    }
    if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
      return trimmed
    }
    return "https://$trimmed"
  }

  fun emitCaptureEvent(event: String, payload: JSObject) {
    trigger(event, payload)
  }
}
