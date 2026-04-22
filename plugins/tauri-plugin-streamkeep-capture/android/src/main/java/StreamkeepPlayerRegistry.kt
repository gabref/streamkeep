package app.streamkeep.capture

import android.webkit.CookieManager
import android.webkit.WebResourceRequest
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import java.text.SimpleDateFormat
import java.lang.ref.WeakReference
import java.util.Date
import java.util.Locale
import java.util.TimeZone

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
  private const val REQUEST_SEEN_EVENT = "capture:request-seen"
  private const val MASTER_DETECTED_EVENT = "capture:master-detected"
  private const val STREAM_DETECTED_EVENT = "stream-detected"
  private const val DEBOUNCE_MS = 30_000L

  private var playerRef: WeakReference<StreamkeepPlayerActivity>? = null
  private var pluginRef: WeakReference<StreamkeepCapturePlugin>? = null
  private val seenRequests = mutableMapOf<String, Long>()

  fun attach(player: StreamkeepPlayerActivity) {
    playerRef = WeakReference(player)
  }

  fun attachPlugin(plugin: StreamkeepCapturePlugin) {
    pluginRef = WeakReference(plugin)
  }

  fun detach(player: StreamkeepPlayerActivity) {
    if (playerRef?.get() == player) {
      playerRef = null
    }
  }

  @Synchronized
  fun recordRequest(
    request: WebResourceRequest,
    source: String,
    pageUrl: String?,
    fallbackUserAgent: String?,
    pageTitle: String?,
    openGraphTitle: String?,
    headingTitle: String?
  ) {
    val url = request.url?.toString() ?: return
    val requestType = classifyRequest(url) ?: return
    val debounceKey = "$requestType:$url"
    val now = System.currentTimeMillis()
    val lastSeenAt = seenRequests[debounceKey]
    if (lastSeenAt != null && now - lastSeenAt < DEBOUNCE_MS) {
      return
    }
    seenRequests[debounceKey] = now

    val payload = createPayload(
      request,
      requestType,
      source,
      pageUrl,
      fallbackUserAgent,
      pageTitle,
      openGraphTitle,
      headingTitle,
      now
    )
    val plugin = pluginRef?.get() ?: return
    plugin.emitCaptureEvent(REQUEST_SEEN_EVENT, payload)

    if (requestType == "master") {
      plugin.emitCaptureEvent(MASTER_DETECTED_EVENT, payload)
      plugin.emitCaptureEvent(STREAM_DETECTED_EVENT, payload)
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

  private fun classifyRequest(url: String): String? {
    val lowerUrl = url.lowercase(Locale.US)
    if (lowerUrl.contains("master.m3u8")) {
      return "master"
    }
    if (lowerUrl.contains(".m3u8")) {
      return "playlist"
    }

    val path = try {
      android.net.Uri.parse(url).path?.lowercase(Locale.US)
    } catch (_: Exception) {
      null
    }
    if (path?.endsWith(".ts") == true) {
      return "segment"
    }

    return null
  }

  private fun createPayload(
    request: WebResourceRequest,
    requestType: String,
    source: String,
    pageUrl: String?,
    fallbackUserAgent: String?,
    pageTitle: String?,
    openGraphTitle: String?,
    headingTitle: String?,
    detectedAtMillis: Long
  ): JSObject {
    val url = request.url.toString()
    val headers = request.requestHeaders ?: emptyMap()
    val userAgent = getHeader(headers, "User-Agent") ?: fallbackUserAgent
    val referer = getHeader(headers, "Referer") ?: getHeader(headers, "Referrer")
    val cookies = CookieManager.getInstance().getCookie(url)
    val payload = JSObject()
    payload.put("url", url)
    payload.put("requestUrl", url)
    payload.put("masterUrl", if (requestType == "master") url else null)
    payload.put("pageUrl", pageUrl)
    payload.put("referer", referer)
    payload.put("userAgent", userAgent)
    payload.put("cookies", cookies)
    payload.put("pageTitle", cleanTitle(pageTitle))
    payload.put("documentTitle", cleanTitle(pageTitle))
    payload.put("openGraphTitle", cleanTitle(openGraphTitle))
    payload.put("headingTitle", cleanTitle(headingTitle))
    payload.put("titleSuggestion", firstPresent(pageTitle, openGraphTitle, headingTitle))
    payload.put("detectedAt", formatDetectedAt(detectedAtMillis))
    payload.put("source", source)
    payload.put("requestType", requestType)
    payload.put("confidence", if (requestType == "master") "strong" else "candidate")
    return payload
  }

  private fun getHeader(headers: Map<String, String>, name: String): String? {
    return headers.entries.firstOrNull { entry ->
      entry.key.equals(name, ignoreCase = true)
    }?.value
  }

  private fun firstPresent(vararg values: String?): String? {
    return values.firstNotNullOfOrNull { cleanTitle(it) }
  }

  private fun cleanTitle(value: String?): String? {
    val title = value?.trim()?.replace(Regex("\\s+"), " ")
    if (title.isNullOrBlank()) {
      return null
    }
    return title
  }

  private fun formatDetectedAt(value: Long): String {
    val formatter = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.US)
    formatter.timeZone = TimeZone.getTimeZone("UTC")
    return formatter.format(Date(value))
  }
}
