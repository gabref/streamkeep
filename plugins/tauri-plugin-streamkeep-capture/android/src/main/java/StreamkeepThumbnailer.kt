package app.streamkeep.capture

import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import android.os.Build
import java.io.File
import java.io.FileOutputStream

data class StreamkeepThumbnailResult(
  val outputPath: String,
  val outputBytes: Long
)

object StreamkeepThumbnailer {
  fun createThumbnail(inputPath: String, outputPath: String): StreamkeepThumbnailResult {
    val inputFile = File(inputPath)
    require(inputFile.exists()) { "Input MP4 file does not exist: $inputPath" }

    val outputFile = File(outputPath)
    outputFile.parentFile?.mkdirs()
    if (outputFile.exists()) {
      outputFile.delete()
    }

    val retriever = MediaMetadataRetriever()
    val bitmap = try {
      retriever.setDataSource(inputPath)
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
        retriever.getScaledFrameAtTime(
          1_000_000L,
          MediaMetadataRetriever.OPTION_CLOSEST_SYNC,
          640,
          360
        )
      } else {
        retriever.frameAtTime
      }
    } finally {
      retriever.release()
    } ?: throw IllegalStateException("No thumbnail frame was available")

    FileOutputStream(outputFile).use { output ->
      bitmap.compress(Bitmap.CompressFormat.JPEG, 82, output)
    }

    return StreamkeepThumbnailResult(
      outputPath = outputFile.absolutePath,
      outputBytes = outputFile.length()
    )
  }
}
