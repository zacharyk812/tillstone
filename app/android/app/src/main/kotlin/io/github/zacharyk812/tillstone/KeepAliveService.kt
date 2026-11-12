package io.github.zacharyk812.tillstone

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper

/**
 * Keeps Tillstone running so sync, printing and kitchen tickets keep working
 * when the app isn't on screen (PLAN §13 "Keeping devices alive").
 *
 * It also records a heartbeat every minute. The M0 proof checks the largest
 * gap between heartbeats after a day in the background: a gap means Android
 * paused or killed the app.
 */
class KeepAliveService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private var multicastLock: WifiManager.MulticastLock? = null

    private val heartbeat = object : Runnable {
        override fun run() {
            recordHeartbeat(this@KeepAliveService)
            handler.postDelayed(this, HEARTBEAT_MS)
        }
    }

    override fun onCreate() {
        super.onCreate()
        val prefs = prefs(this)
        if (!prefs.contains(KEY_STARTED)) {
            prefs.edit().putLong(KEY_STARTED, System.currentTimeMillis()).apply()
        }
        // Android drops multicast packets unless an app holds this lock, and
        // local discovery (mDNS) depends on them.
        val wifi = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
        multicastLock = wifi.createMulticastLock("tillstone-discovery").apply {
            setReferenceCounted(false)
            acquire()
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        handler.removeCallbacks(heartbeat)
        handler.post(heartbeat)
        return START_STICKY
    }

    override fun onDestroy() {
        handler.removeCallbacks(heartbeat)
        multicastLock?.release()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun buildNotification(): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            manager.createNotificationChannel(
                NotificationChannel(CHANNEL_ID, "Tillstone running", NotificationManager.IMPORTANCE_LOW),
            )
        }
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle("Tillstone is running")
            .setContentText("Keeping sales, sync and printing available")
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setOngoing(true)
            .build()
    }

    companion object {
        private const val CHANNEL_ID = "tillstone-keepalive"
        private const val NOTIFICATION_ID = 1
        private const val HEARTBEAT_MS = 60_000L
        private const val PREFS = "tillstone-keepalive"
        private const val KEY_STARTED = "started"
        private const val KEY_LAST = "last"
        private const val KEY_COUNT = "count"
        private const val KEY_MAX_GAP = "maxGap"

        private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

        fun start(context: Context) {
            val intent = Intent(context, KeepAliveService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun recordHeartbeat(context: Context) {
            val p = prefs(context)
            val now = System.currentTimeMillis()
            val last = p.getLong(KEY_LAST, 0L)
            val gap = if (last == 0L) 0L else now - last
            p.edit()
                .putLong(KEY_LAST, now)
                .putLong(KEY_COUNT, p.getLong(KEY_COUNT, 0L) + 1)
                .putLong(KEY_MAX_GAP, maxOf(p.getLong(KEY_MAX_GAP, 0L), gap))
                .apply()
        }

        /** Survival statistics for the proof screen. */
        fun status(context: Context): Map<String, Long> {
            val p = prefs(context)
            return mapOf(
                "startedMs" to p.getLong(KEY_STARTED, 0L),
                "lastHeartbeatMs" to p.getLong(KEY_LAST, 0L),
                "heartbeats" to p.getLong(KEY_COUNT, 0L),
                "maxGapMs" to p.getLong(KEY_MAX_GAP, 0L),
            )
        }
    }
}
