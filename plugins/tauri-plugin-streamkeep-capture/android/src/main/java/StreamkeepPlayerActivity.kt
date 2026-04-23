package app.streamkeep.capture

import android.annotation.SuppressLint
import android.app.Activity
import android.content.Context
import android.graphics.Color
import android.graphics.Rect
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.text.TextUtils
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import android.webkit.CookieManager
import android.webkit.ServiceWorkerClient
import android.webkit.ServiceWorkerController
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import app.tauri.plugin.JSObject
import org.json.JSONArray
import org.json.JSONObject

class StreamkeepPlayerActivity : Activity() {
  private lateinit var rootFrame: FrameLayout
  private lateinit var webView: WebView
  private lateinit var urlField: EditText
  private lateinit var titleView: TextView
  private lateinit var progress: ProgressBar
  private lateinit var detectedPane: LinearLayout
  private lateinit var detectedTitleView: TextView
  private lateinit var detectedFileNameField: EditText
  private lateinit var detectedCancelButton: Button
  private lateinit var detectedDownloadButton: Button
  private var detectedPaneKeyboardOffset = 0
  private var detectedPayload: JSObject? = null
  private var loading = false
  private var currentTitle: String? = null
  private var currentPageUrl: String? = null
  private var currentUserAgent: String? = null
  private var currentDocumentTitle: String? = null
  private var currentOpenGraphTitle: String? = null
  private var currentHeadingTitle: String? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    window.statusBarColor = Color.rgb(15, 19, 20)
    window.navigationBarColor = Color.rgb(15, 19, 20)
    window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)

    setContentView(createLayout())
    configureWebView()

    StreamkeepPlayerRegistry.attach(this)
    val startUrl = intent.getStringExtra(EXTRA_INITIAL_URL) ?: DEFAULT_URL
    loadUrl(startUrl)
  }

  override fun onDestroy() {
    StreamkeepPlayerRegistry.detach(this)
    webView.destroy()
    super.onDestroy()
  }

  override fun onBackPressed() {
    if (::webView.isInitialized && webView.canGoBack()) {
      webView.goBack()
      return
    }
    super.onBackPressed()
  }

  fun loadUrl(url: String) {
    val normalized = normalizeUrl(url)
    clearPageMetadata()
    currentPageUrl = normalized
    persistLastUrl(normalized)
    urlField.setText(normalized)
    webView.loadUrl(normalized)
  }

  fun reload() {
    webView.reload()
  }

  fun goBack() {
    if (webView.canGoBack()) {
      webView.goBack()
    }
  }

  fun goForward() {
    if (webView.canGoForward()) {
      webView.goForward()
    }
  }

  fun snapshot() = StreamkeepPlayerSnapshot(
    supported = true,
    visible = true,
    loading = loading,
    url = webView.url,
    title = currentTitle,
    canGoBack = webView.canGoBack(),
    canGoForward = webView.canGoForward()
  ).toJsObject()

  fun showDetectedStream(payload: JSObject) {
    runOnUiThread {
      detectedPayload = payload
      val title = firstNonBlank(payload.optString("titleSuggestion"), null)
        ?: firstNonBlank(payload.optString("documentTitle"), null)
        ?: firstNonBlank(payload.optString("pageTitle"), null)
        ?: "Streamkeep capture"
      detectedTitleView.text = title
      detectedFileNameField.setText(sanitizeFileStem(title))
      detectedCancelButton.isEnabled = true
      detectedDownloadButton.isEnabled = true
      detectedDownloadButton.text = "Download MP4"
      showDetectedPane()
    }
  }

  private fun createLayout(): View {
    rootFrame = FrameLayout(this).apply {
      setBackgroundColor(Color.rgb(15, 19, 20))
      layoutParams = FrameLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.MATCH_PARENT
      )
    }
    installKeyboardAwareDetectedPaneLayout()

    val root = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setBackgroundColor(Color.rgb(15, 19, 20))
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.MATCH_PARENT
      )
    }

    titleView = TextView(this).apply {
      text = "Streamkeep Player"
      setTextColor(Color.rgb(245, 247, 247))
      setTypeface(typeface, Typeface.BOLD)
      textSize = 16f
      gravity = Gravity.CENTER_VERTICAL
      maxLines = 1
      ellipsize = TextUtils.TruncateAt.END
      setPadding(dp(14), statusBarHeight() + dp(8), dp(14), dp(2))
    }
    root.addView(titleView, LinearLayout.LayoutParams(
      ViewGroup.LayoutParams.MATCH_PARENT,
      statusBarHeight() + dp(42)
    ))

    val controls = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER_VERTICAL
      setPadding(dp(8), dp(4), dp(8), dp(8))
    }

    controls.addView(iconToolbarButton("Back", "\u2039") { goBack() }, LinearLayout.LayoutParams(0, dp(40), 1f))
    controls.addView(iconToolbarButton("Forward", "\u203a") { goForward() }, LinearLayout.LayoutParams(0, dp(40), 1f))
    controls.addView(iconToolbarButton("Reload", "\u21bb") { reload() }, LinearLayout.LayoutParams(0, dp(40), 1f))
    controls.addView(iconToolbarButton("Close", "\u2715") { finish() }, LinearLayout.LayoutParams(0, dp(40), 1f))
    root.addView(controls)

    val addressBar = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER_VERTICAL
      setPadding(dp(8), 0, dp(8), dp(8))
    }

    urlField = EditText(this).apply {
      setSingleLine(true)
      setTextColor(Color.rgb(245, 247, 247))
      setHintTextColor(Color.rgb(170, 182, 185))
      background = roundedBackground(Color.rgb(16, 23, 25), Color.rgb(51, 65, 69))
      textSize = 14f
      hint = "https://"
      imeOptions = EditorInfo.IME_ACTION_GO
      inputType = android.text.InputType.TYPE_TEXT_VARIATION_URI
      setPadding(dp(10), 0, dp(10), 0)
      setOnEditorActionListener { _, actionId, event ->
        val isEnter = event?.keyCode == KeyEvent.KEYCODE_ENTER && event.action == KeyEvent.ACTION_UP
        if (actionId == EditorInfo.IME_ACTION_GO || isEnter) {
          loadUrl(text.toString())
          true
        } else {
          false
        }
      }
    }
    addressBar.addView(urlField, LinearLayout.LayoutParams(0, dp(46), 1f))
    addressBar.addView(toolbarButton("Go") { loadUrl(urlField.text.toString()) }, LinearLayout.LayoutParams(dp(86), dp(46)))
    root.addView(addressBar)

    progress = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply {
      max = 100
      visibility = View.GONE
    }
    root.addView(progress, LinearLayout.LayoutParams(
      ViewGroup.LayoutParams.MATCH_PARENT,
      dp(4)
    ))

    webView = WebView(this)
    root.addView(webView, LinearLayout.LayoutParams(
      ViewGroup.LayoutParams.MATCH_PARENT,
      0,
      1f
    ))

    rootFrame.addView(root)
    detectedPane = createDetectedPane()
    rootFrame.addView(
      detectedPane,
      FrameLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT,
        Gravity.BOTTOM
      )
    )

    return rootFrame
  }

  private fun showDetectedPane() {
    detectedPane.clearAnimation()
    if (detectedPane.visibility != View.VISIBLE) {
      detectedPane.alpha = 0f
      detectedPane.translationY = dp(22).toFloat()
      detectedPane.visibility = View.VISIBLE
    }
    detectedPane.animate()
      .alpha(1f)
      .translationY(0f)
      .setDuration(180L)
      .start()
  }

  private fun hideDetectedPane() {
    detectedPane.clearAnimation()
    detectedPane.animate()
      .alpha(0f)
      .translationY(dp(22).toFloat())
      .setDuration(160L)
      .withEndAction {
        detectedPane.visibility = View.GONE
        detectedPane.translationY = 0f
        detectedPane.alpha = 1f
      }
      .start()
  }

  private fun installKeyboardAwareDetectedPaneLayout() {
    rootFrame.viewTreeObserver.addOnGlobalLayoutListener {
      val visibleFrame = Rect()
      rootFrame.getWindowVisibleDisplayFrame(visibleFrame)
      val rootHeight = rootFrame.rootView.height
      val keyboardOffset = (rootHeight - visibleFrame.bottom).coerceAtLeast(0)
      val threshold = (rootHeight * 0.12f).toInt()
      val bottomMargin = if (keyboardOffset > threshold) keyboardOffset else 0

      if (bottomMargin != detectedPaneKeyboardOffset && ::detectedPane.isInitialized) {
        detectedPaneKeyboardOffset = bottomMargin
        val params = detectedPane.layoutParams as FrameLayout.LayoutParams
        params.bottomMargin = bottomMargin
        detectedPane.layoutParams = params
      }
    }
  }

  @SuppressLint("SetJavaScriptEnabled")
  private fun configureWebView() {
    CookieManager.getInstance().setAcceptCookie(true)
    CookieManager.getInstance().setAcceptThirdPartyCookies(webView, true)

    webView.settings.apply {
      javaScriptEnabled = true
      domStorageEnabled = true
      mediaPlaybackRequiresUserGesture = false
      loadWithOverviewMode = true
      useWideViewPort = true
      builtInZoomControls = false
      displayZoomControls = false
      userAgentString = "$userAgentString Streamkeep/0.1"
      currentUserAgent = userAgentString
    }

    webView.webViewClient = object : WebViewClient() {
      override fun shouldOverrideUrlLoading(
        view: WebView,
        request: WebResourceRequest
      ): Boolean {
        return false
      }

      override fun shouldInterceptRequest(
        view: WebView,
        request: WebResourceRequest
      ): WebResourceResponse? {
        recordRequest(request, "webview")
        return null
      }

      override fun onPageStarted(view: WebView, url: String?, favicon: android.graphics.Bitmap?) {
        loading = true
        progress.visibility = View.VISIBLE
        clearPageMetadata()
        if (url != null) {
          currentPageUrl = url
          persistLastUrl(url)
          urlField.setText(url)
        }
        super.onPageStarted(view, url, favicon)
      }

      override fun onPageFinished(view: WebView, url: String?) {
        loading = false
        progress.visibility = View.GONE
        CookieManager.getInstance().flush()
        if (url != null) {
          currentPageUrl = url
          persistLastUrl(url)
          urlField.setText(url)
        }
        refreshPageMetadata(view)
        super.onPageFinished(view, url)
      }
    }

    webView.webChromeClient = object : WebChromeClient() {
      override fun onProgressChanged(view: WebView, newProgress: Int) {
        progress.progress = newProgress
        progress.visibility = if (newProgress in 1..99) View.VISIBLE else View.GONE
        super.onProgressChanged(view, newProgress)
      }

      override fun onReceivedTitle(view: WebView, title: String?) {
        currentTitle = title
        currentDocumentTitle = title
        titleView.text = if (title.isNullOrBlank()) "Streamkeep Player" else "Streamkeep - $title"
        super.onReceivedTitle(view, title)
      }
    }

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
      ServiceWorkerController.getInstance().setServiceWorkerClient(
        object : ServiceWorkerClient() {
          override fun shouldInterceptRequest(request: WebResourceRequest): WebResourceResponse? {
            recordRequest(request, "service-worker")
            return null
          }
        }
      )
    }
  }

  private fun recordRequest(request: WebResourceRequest, source: String) {
    StreamkeepPlayerRegistry.recordRequest(
      request,
      source,
      currentPageUrl,
      currentUserAgent,
      currentDocumentTitle ?: currentTitle,
      currentOpenGraphTitle,
      currentHeadingTitle
    )
  }

  private fun clearPageMetadata() {
    currentTitle = null
    currentDocumentTitle = null
    currentOpenGraphTitle = null
    currentHeadingTitle = null
  }

  private fun refreshPageMetadata(view: WebView) {
    val script = """
      (function () {
        var og = document.querySelector('meta[property="og:title"], meta[name="og:title"]');
        var heading = document.querySelector('h1, h2, [data-title], [aria-label]');
        return JSON.stringify({
          title: document.title || '',
          openGraphTitle: og && og.content ? og.content : '',
          headingTitle: heading ? (heading.textContent || heading.getAttribute('aria-label') || '') : ''
        });
      })();
    """.trimIndent()

    view.evaluateJavascript(script) { rawValue ->
      val jsonText = decodeJavascriptString(rawValue)
      if (jsonText.isNullOrBlank()) {
        return@evaluateJavascript
      }
      try {
        val metadata = JSONObject(jsonText)
        currentDocumentTitle = firstNonBlank(metadata.optString("title"), currentDocumentTitle)
        currentOpenGraphTitle = firstNonBlank(metadata.optString("openGraphTitle"), currentOpenGraphTitle)
        currentHeadingTitle = firstNonBlank(metadata.optString("headingTitle"), currentHeadingTitle)
      } catch (_: Exception) {
      }
    }
  }

  private fun decodeJavascriptString(rawValue: String?): String? {
    if (rawValue == null || rawValue == "null") {
      return null
    }
    return try {
      JSONArray("[$rawValue]").optString(0)
    } catch (_: Exception) {
      rawValue.trim('"')
    }
  }

  private fun firstNonBlank(value: String?, fallback: String?): String? {
    val trimmed = value?.trim()
    if (!trimmed.isNullOrBlank()) {
      return trimmed
    }
    return fallback
  }

  private fun toolbarButton(label: String, icon: String? = null, onClick: () -> Unit): Button {
    val textValue = if (icon.isNullOrBlank()) label else "$icon $label"
    return Button(this).apply {
      text = textValue
      isAllCaps = false
      minWidth = dp(48)
      minHeight = dp(40)
      setTextColor(Color.rgb(245, 247, 247))
      setTypeface(typeface, Typeface.BOLD)
      textSize = 11f
      background = roundedBackground(Color.rgb(32, 40, 43), Color.rgb(51, 65, 69))
      setOnClickListener { onClick() }
    }
  }

  private fun iconToolbarButton(label: String, icon: String, onClick: () -> Unit): TextView {
    return TextView(this).apply {
      text = icon
      contentDescription = label
      minWidth = dp(40)
      minHeight = dp(40)
      setTextColor(Color.rgb(245, 247, 247))
      setTypeface(typeface, Typeface.BOLD)
      textSize = 22f
      gravity = Gravity.CENTER
      includeFontPadding = false
      isClickable = true
      isFocusable = true
      background = roundedBackground(Color.rgb(32, 40, 43), Color.rgb(51, 65, 69))
      setOnClickListener { onClick() }
    }
  }

  private fun createDetectedPane(): LinearLayout {
    return LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      visibility = View.GONE
      setPadding(dp(14), dp(12), dp(14), dp(14))
      background = roundedBackground(Color.rgb(23, 29, 31), Color.rgb(70, 87, 92), dp(14).toFloat())
      elevation = dp(10).toFloat()

      addView(TextView(this@StreamkeepPlayerActivity).apply {
        text = "Video detected"
        setTextColor(Color.rgb(137, 223, 168))
        setTypeface(typeface, Typeface.BOLD)
        textSize = 12f
      })

      detectedTitleView = TextView(this@StreamkeepPlayerActivity).apply {
        setTextColor(Color.rgb(245, 247, 247))
        setTypeface(typeface, Typeface.BOLD)
        textSize = 16f
        maxLines = 2
        setPadding(0, dp(4), 0, dp(10))
      }
      addView(detectedTitleView)

      detectedFileNameField = EditText(this@StreamkeepPlayerActivity).apply {
        setSingleLine(true)
        setTextColor(Color.rgb(245, 247, 247))
        setHintTextColor(Color.rgb(170, 182, 185))
        background = roundedBackground(Color.rgb(16, 23, 25), Color.rgb(51, 65, 69))
        hint = "File name"
        textSize = 14f
        setPadding(dp(10), 0, dp(10), 0)
      }
      addView(detectedFileNameField, LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        dp(46)
      ))

      val actions = LinearLayout(this@StreamkeepPlayerActivity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, dp(10), 0, 0)
      }
      detectedCancelButton = toolbarButton("Not now") {
        hideDetectedPane()
      }
      actions.addView(detectedCancelButton, LinearLayout.LayoutParams(0, dp(44), 1f))
      detectedDownloadButton = toolbarButton("Download MP4") {
        val payload = detectedPayload ?: return@toolbarButton
        detectedCancelButton.isEnabled = false
        detectedDownloadButton.isEnabled = false
        detectedDownloadButton.text = "Starting..."
        Toast.makeText(this@StreamkeepPlayerActivity, "Streamkeep download started", Toast.LENGTH_SHORT).show()
        StreamkeepPlayerRegistry.requestDownload(payload, detectedFileNameField.text.toString())
        hideDetectedPane()
      }
      actions.addView(detectedDownloadButton, LinearLayout.LayoutParams(0, dp(44), 1.2f))
      addView(actions)
    }
  }

  private fun roundedBackground(
    fillColor: Int,
    strokeColor: Int,
    radius: Float = dp(8).toFloat()
  ): GradientDrawable {
    return GradientDrawable().apply {
      setColor(fillColor)
      cornerRadius = radius
      setStroke(dp(1), strokeColor)
    }
  }

  private fun normalizeUrl(value: String): String {
    val trimmed = Uri.decode(value).trim()
    if (trimmed.isBlank()) {
      return currentPageUrl ?: DEFAULT_URL
    }
    if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
      return trimmed
    }
    if (looksLikeLocalAddress(trimmed)) {
      return "http://$trimmed"
    }
    return "https://$trimmed"
  }

  private fun looksLikeLocalAddress(value: String): Boolean {
    return value.startsWith("localhost", ignoreCase = true) ||
      value.startsWith("127.") ||
      value.startsWith("10.") ||
      value.startsWith("192.168.") ||
      Regex("""^172\.(1[6-9]|2\d|3[0-1])\.""").containsMatchIn(value)
  }

  private fun sanitizeFileStem(value: String): String {
    val cleaned = value
      .replace(Regex("""[<>:"/\\|?*\u0000-\u001F]"""), " ")
      .replace(Regex("""\s+"""), " ")
      .trim('.', ' ')
    return cleaned.ifBlank { "Streamkeep capture" }.removeSuffix(".mp4")
  }

  private fun persistLastUrl(value: String) {
    getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
      .edit()
      .putString(LAST_URL_KEY, value)
      .apply()
  }

  private fun dp(value: Int): Int {
    return (value * resources.displayMetrics.density).toInt()
  }

  private fun statusBarHeight(): Int {
    val resourceId = resources.getIdentifier("status_bar_height", "dimen", "android")
    return if (resourceId > 0) {
      resources.getDimensionPixelSize(resourceId)
    } else {
      0
    }
  }

  companion object {
    const val EXTRA_INITIAL_URL = "app.streamkeep.capture.initial_url"
    const val DEFAULT_URL = "https://example.com"
    private const val PREFERENCES_NAME = "streamkeep-player"
    private const val LAST_URL_KEY = "last-url"

    fun lastUrl(context: Context): String {
      return context
        .getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
        .getString(LAST_URL_KEY, DEFAULT_URL)
        ?: DEFAULT_URL
    }
  }
}
