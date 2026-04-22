package app.streamkeep.capture

import android.app.Activity
import android.content.ContentValues
import android.media.MediaScannerConnection
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import java.io.File

data class StreamkeepPublishResult(
  val contentUri: String,
  val displayName: String,
  val relativePath: String,
  val outputBytes: Long
)

object StreamkeepMediaStorePublisher {
  private const val MIME_TYPE = "video/mp4"
  private const val STREAMKEEP_FOLDER = "Streamkeep"

  fun publishToDownloads(
    activity: Activity,
    inputPath: String,
    displayName: String
  ): StreamkeepPublishResult {
    val inputFile = File(inputPath)
    require(inputFile.exists()) { "Input MP4 file does not exist: $inputPath" }
    require(inputFile.length() > 0L) { "Input MP4 file is empty: $inputPath" }

    val safeDisplayName = sanitizeDisplayName(displayName)
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      publishScoped(activity, inputFile, safeDisplayName)
    } else {
      publishLegacy(activity, inputFile, safeDisplayName)
    }
  }

  private fun publishScoped(
    activity: Activity,
    inputFile: File,
    displayName: String
  ): StreamkeepPublishResult {
    val resolver = activity.contentResolver
    val relativePath = "${Environment.DIRECTORY_DOWNLOADS}/$STREAMKEEP_FOLDER"
    val values = ContentValues().apply {
      put(MediaStore.MediaColumns.DISPLAY_NAME, displayName)
      put(MediaStore.MediaColumns.MIME_TYPE, MIME_TYPE)
      put(MediaStore.MediaColumns.RELATIVE_PATH, relativePath)
      put(MediaStore.MediaColumns.IS_PENDING, 1)
    }
    val collection = MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
    val uri = requireNotNull(resolver.insert(collection, values)) {
      "Failed to create MediaStore Downloads entry"
    }

    try {
      resolver.openOutputStream(uri, "w").use { output ->
        requireNotNull(output) { "Failed to open MediaStore output stream" }
        inputFile.inputStream().use { input ->
          input.copyTo(output)
        }
      }

      values.clear()
      values.put(MediaStore.MediaColumns.IS_PENDING, 0)
      resolver.update(uri, values, null, null)
    } catch (ex: Exception) {
      resolver.delete(uri, null, null)
      throw ex
    }

    return StreamkeepPublishResult(
      contentUri = uri.toString(),
      displayName = displayName,
      relativePath = relativePath,
      outputBytes = inputFile.length()
    )
  }

  private fun publishLegacy(
    activity: Activity,
    inputFile: File,
    displayName: String
  ): StreamkeepPublishResult {
    val relativePath = "${Environment.DIRECTORY_DOWNLOADS}/$STREAMKEEP_FOLDER"
    val downloadsDir = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)
    val outputDir = File(downloadsDir, STREAMKEEP_FOLDER)
    outputDir.mkdirs()

    val outputFile = uniqueFile(outputDir, displayName)
    inputFile.copyTo(outputFile, overwrite = false)
    MediaScannerConnection.scanFile(
      activity,
      arrayOf(outputFile.absolutePath),
      arrayOf(MIME_TYPE),
      null
    )

    return StreamkeepPublishResult(
      contentUri = Uri.fromFile(outputFile).toString(),
      displayName = outputFile.name,
      relativePath = relativePath,
      outputBytes = outputFile.length()
    )
  }

  private fun sanitizeDisplayName(value: String): String {
    val cleaned = value
      .replace(Regex("""[<>:"/\\|?*\u0000-\u001F]"""), " ")
      .split(Regex("""\s+"""))
      .filter { it.isNotBlank() }
      .joinToString(" ")
      .trim('.', ' ')

    val withFallback = cleaned.ifBlank { "Streamkeep capture.mp4" }
    return if (withFallback.lowercase().endsWith(".mp4")) {
      withFallback
    } else {
      "$withFallback.mp4"
    }
  }

  private fun uniqueFile(directory: File, displayName: String): File {
    val first = File(directory, displayName)
    if (!first.exists()) {
      return first
    }

    val stem = displayName.removeSuffix(".mp4")
    for (index in 1 until 1000) {
      val candidate = File(directory, "$stem ($index).mp4")
      if (!candidate.exists()) {
        return candidate
      }
    }

    return File(directory, "$stem (999).mp4")
  }
}
