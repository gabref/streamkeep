package app.streamkeep.capture

import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import java.lang.ref.WeakReference

data class StreamkeepPlayerSnapshot(
  val supported: Boolean,
  val visible: Boolean,
  val loading: Boolean,
  val url: String?,
  val title: String?,
  val canGoBack: Boolean,
  val canGoForward: Boolean
) {
  fun toJsObject(): JSObject {
    val state = JSObject()
    state.put("supported", supported)
    state.put("visible", visible)
    state.put("loading", loading)
    state.put("url", url)
    state.put("title", title)
    state.put("canGoBack", canGoBack)
    state.put("canGoForward", canGoForward)
    return state
  }
}

object StreamkeepPlayerRegistry {
  private var playerRef: WeakReference<StreamkeepPlayerActivity>? = null

  fun attach(player: StreamkeepPlayerActivity) {
    playerRef = WeakReference(player)
  }

  fun detach(player: StreamkeepPlayerActivity) {
    if (playerRef?.get() == player) {
      playerRef = null
    }
  }

  fun snapshot(): JSObject {
    val player = playerRef?.get()
    if (player != null) {
      return player.snapshot()
    }
    return StreamkeepPlayerSnapshot(
      supported = true,
      visible = false,
      loading = false,
      url = null,
      title = null,
      canGoBack = false,
      canGoForward = false
    ).toJsObject()
  }

  fun runOnPlayer(invoke: Invoke, action: (StreamkeepPlayerActivity) -> Unit) {
    val player = playerRef?.get()
    if (player == null) {
      invoke.reject("Streamkeep player is not open")
      return
    }

    player.runOnUiThread {
      try {
        action(player)
        invoke.resolve(player.snapshot())
      } catch (ex: Exception) {
        invoke.reject(ex.message ?: "Streamkeep player command failed")
      }
    }
  }
}
