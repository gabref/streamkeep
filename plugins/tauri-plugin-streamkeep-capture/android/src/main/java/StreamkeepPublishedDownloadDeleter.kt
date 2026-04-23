package app.streamkeep.capture

import android.app.Activity
import android.net.Uri
import java.io.File

object StreamkeepPublishedDownloadDeleter {
  fun delete(activity: Activity, contentUri: String) {
    val uri = Uri.parse(contentUri)
    if (uri.scheme == "file") {
      uri.path?.let { path -> File(path).delete() }
      return
    }

    activity.contentResolver.delete(uri, null, null)
  }
}
