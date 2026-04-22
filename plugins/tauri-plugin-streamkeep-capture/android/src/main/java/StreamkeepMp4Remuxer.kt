package app.streamkeep.capture

import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import android.media.MediaMuxer
import java.io.File
import java.nio.ByteBuffer

data class StreamkeepRemuxResult(
  val outputPath: String,
  val trackCount: Int,
  val outputBytes: Long
)

object StreamkeepMp4Remuxer {
  private const val DEFAULT_BUFFER_SIZE = 1024 * 1024

  fun remuxToMp4(inputPath: String, outputPath: String): StreamkeepRemuxResult {
    val inputFile = File(inputPath)
    require(inputFile.exists()) { "Input media file does not exist: $inputPath" }

    val outputFile = File(outputPath)
    outputFile.parentFile?.mkdirs()
    if (outputFile.exists()) {
      outputFile.delete()
    }

    val extractor = MediaExtractor()
    val muxer = MediaMuxer(outputPath, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4)
    var muxerStarted = false

    try {
      extractor.setDataSource(inputPath)
      val trackMap = mutableMapOf<Int, Int>()
      var bufferSize = DEFAULT_BUFFER_SIZE

      for (trackIndex in 0 until extractor.trackCount) {
        val format = extractor.getTrackFormat(trackIndex)
        val mime = format.getString(MediaFormat.KEY_MIME).orEmpty()
        if (!mime.startsWith("video/") && !mime.startsWith("audio/")) {
          continue
        }

        extractor.selectTrack(trackIndex)
        val muxerTrackIndex = muxer.addTrack(format)
        trackMap[trackIndex] = muxerTrackIndex
        bufferSize = maxOf(bufferSize, maxInputSize(format))
      }

      require(trackMap.isNotEmpty()) { "No audio or video tracks found in input media" }

      muxer.start()
      muxerStarted = true

      val buffer = ByteBuffer.allocate(bufferSize)
      val bufferInfo = MediaCodec.BufferInfo()

      while (true) {
        buffer.clear()
        val sampleSize = extractor.readSampleData(buffer, 0)
        if (sampleSize < 0) {
          break
        }

        val sampleTrackIndex = extractor.sampleTrackIndex
        val muxerTrackIndex = trackMap[sampleTrackIndex]
        if (muxerTrackIndex != null) {
          bufferInfo.set(
            0,
            sampleSize,
            maxOf(extractor.sampleTime, 0L),
            sampleFlags(extractor.sampleFlags)
          )
          muxer.writeSampleData(muxerTrackIndex, buffer, bufferInfo)
        }
        extractor.advance()
      }
    } finally {
      extractor.release()
      if (muxerStarted) {
        muxer.stop()
      }
      muxer.release()
    }

    return StreamkeepRemuxResult(
      outputPath = outputFile.absolutePath,
      trackCount = trackCount(outputPath),
      outputBytes = outputFile.length()
    )
  }

  private fun maxInputSize(format: MediaFormat): Int {
    return if (format.containsKey(MediaFormat.KEY_MAX_INPUT_SIZE)) {
      format.getInteger(MediaFormat.KEY_MAX_INPUT_SIZE)
    } else {
      DEFAULT_BUFFER_SIZE
    }
  }

  private fun sampleFlags(flags: Int): Int {
    var bufferFlags = 0
    if (flags and MediaExtractor.SAMPLE_FLAG_SYNC != 0) {
      bufferFlags = bufferFlags or MediaCodec.BUFFER_FLAG_KEY_FRAME
    }
    return bufferFlags
  }

  private fun trackCount(path: String): Int {
    val extractor = MediaExtractor()
    return try {
      extractor.setDataSource(path)
      extractor.trackCount
    } finally {
      extractor.release()
    }
  }
}
