package app.streamkeep.capture

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder

class StreamkeepDownloadService : Service() {
  override fun onCreate() {
    super.onCreate()
    ensureNotificationChannel()
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    startForeground(NOTIFICATION_ID, notification())
    return START_STICKY
  }

  override fun onBind(intent: Intent?): IBinder? = null

  private fun notification(): Notification {
    val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      Notification.Builder(this, CHANNEL_ID)
    } else {
      @Suppress("DEPRECATION")
      Notification.Builder(this)
    }

    return builder
      .setContentTitle("Streamkeep")
      .setContentText("Download in progress")
      .setSmallIcon(applicationInfo.icon)
      .setOngoing(true)
      .build()
  }

  private fun ensureNotificationChannel() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
      return
    }

    val manager = getSystemService(NotificationManager::class.java)
    val channel = NotificationChannel(
      CHANNEL_ID,
      "Streamkeep downloads",
      NotificationManager.IMPORTANCE_LOW
    )
    manager.createNotificationChannel(channel)
  }

  companion object {
    private const val CHANNEL_ID = "streamkeep-downloads"
    private const val NOTIFICATION_ID = 1001

    fun start(context: Context) {
      val intent = Intent(context, StreamkeepDownloadService::class.java)
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        context.startForegroundService(intent)
      } else {
        context.startService(intent)
      }
    }

    fun stop(context: Context) {
      context.stopService(Intent(context, StreamkeepDownloadService::class.java))
    }
  }
}
