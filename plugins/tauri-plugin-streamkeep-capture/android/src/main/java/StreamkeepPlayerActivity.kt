package app.streamkeep.capture

import android.annotation.SuppressLint
import android.app.Activity
import android.graphics.Color
import android.graphics.Typeface
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
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
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import org.json.JSONArray
import org.json.JSONObject

class StreamkeepPlayerActivity : Activity() {
  private lateinit var webView: WebView
  private lateinit var urlField: EditText
  private lateinit var titleView: TextView
  private lateinit var progress: ProgressBar
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

  private fun createLayout(): View {
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
      setPadding(dp(14), dp(8), dp(14), dp(2))
    }
    root.addView(titleView, LinearLayout.LayoutParams(
      ViewGroup.LayoutParams.MATCH_PARENT,
      dp(40)
    ))

    val controls = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER_VERTICAL
      setPadding(dp(8), dp(4), dp(8), dp(8))
    }

    controls.addView(toolbarButton("Back") { goBack() }, LinearLayout.LayoutParams(0, dp(46), 1f))
    controls.addView(toolbarButton("Forward") { goForward() }, LinearLayout.LayoutParams(0, dp(46), 1f))
    controls.addView(toolbarButton("Reload") { reload() }, LinearLayout.LayoutParams(0, dp(46), 1f))
    controls.addView(toolbarButton("Close") { finish() }, LinearLayout.LayoutParams(0, dp(46), 1f))
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
      setBackgroundColor(Color.rgb(32, 40, 43))
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

    return root
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

  private fun toolbarButton(label: String, onClick: () -> Unit): Button {
    return Button(this).apply {
      text = label
      minWidth = dp(48)
      minHeight = dp(46)
      setOnClickListener { onClick() }
    }
  }

  private fun normalizeUrl(value: String): String {
    val trimmed = value.trim()
    if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
      return trimmed
    }
    return "https://${Uri.encode(trimmed)}"
  }

  private fun dp(value: Int): Int {
    return (value * resources.displayMetrics.density).toInt()
  }

  companion object {
    const val EXTRA_INITIAL_URL = "app.streamkeep.capture.initial_url"
    const val DEFAULT_URL = "https://example.com"
  }
}
