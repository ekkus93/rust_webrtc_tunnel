package com.phillipchin.webrtctunnel.notification

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import com.phillipchin.webrtctunnel.MainActivity
import com.phillipchin.webrtctunnel.model.ServiceState

class NotificationController(private val context: Context) {
    companion object {
        const val CHANNEL_STATUS = "tunnel_status"
        const val CHANNEL_ERRORS = "tunnel_errors"
        const val NOTIFICATION_ID = 1001
    }

    fun ensureChannels() {
        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val status = NotificationChannel(CHANNEL_STATUS, "Tunnel Status", NotificationManager.IMPORTANCE_LOW)
        val errors = NotificationChannel(CHANNEL_ERRORS, "Tunnel Errors", NotificationManager.IMPORTANCE_HIGH)
        manager.createNotificationChannels(listOf(status, errors))
    }

    fun buildStatusNotification(state: ServiceState, body: String): android.app.Notification {
        val openIntent = PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val action = PendingIntent.getService(
            context,
            1,
            Intent(context, com.phillipchin.webrtctunnel.TunnelForegroundService::class.java).apply {
                action = com.phillipchin.webrtctunnel.TunnelForegroundService.ACTION_STOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val title = when (state) {
            ServiceState.PausedMeteredBlocked -> "WebRTC Tunnel paused"
            ServiceState.Error, ServiceState.ConfigInvalid -> "WebRTC Tunnel error"
            else -> "WebRTC Tunnel running"
        }
        return NotificationCompat.Builder(context, CHANNEL_STATUS)
            .setSmallIcon(android.R.drawable.stat_sys_data_sync)
            .setContentTitle(title)
            .setContentText(body)
            .setContentIntent(openIntent)
            .addAction(android.R.drawable.ic_media_pause, "Stop", action)
            .setOngoing(true)
            .build()
    }

    fun show(notification: android.app.Notification) {
        NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, notification)
    }
}
