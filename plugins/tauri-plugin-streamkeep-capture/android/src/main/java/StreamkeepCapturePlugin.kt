package app.streamkeep.capture

import android.app.Activity
import android.content.Intent
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class OpenPlayerArgs {
  var url: String? = null
}

@InvokeArg
class LoadUrlArgs {
  lateinit var url: String
}

@TauriPlugin
class StreamkeepCapturePlugin(private val activity: Activity) : Plugin(activity) {
  init {
    StreamkeepPlayerRegistry.attachPlugin(this)
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

  fun emitCaptureEvent(event: String, payload: app.tauri.plugin.JSObject) {
    trigger(event, payload)
  }
}
