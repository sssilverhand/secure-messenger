package com.privmsg.app.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Binder
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.privmsg.app.MainActivity
import com.privmsg.app.service.WebSocketManager
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * Background service for message synchronization and real-time delivery.
 * Maintains WebSocket connection for instant messaging.
 */
class MessageService : Service() {

    private val binder = MessageBinder()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private var webSocketManager: WebSocketManager? = null
    private var isRunning = false

    private val _connectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    val connectionState: StateFlow<ConnectionState> = _connectionState

    private val _unreadCount = MutableStateFlow(0)
    val unreadCount: StateFlow<Int> = _unreadCount

    companion object {
        const val CHANNEL_ID = "privmsg_message_channel"
        const val NOTIFICATION_ID = 1002

        const val ACTION_START = "com.privmsg.app.MESSAGE_SERVICE_START"
        const val ACTION_STOP = "com.privmsg.app.MESSAGE_SERVICE_STOP"

        const val EXTRA_SERVER_URL = "server_url"
        const val EXTRA_TOKEN = "token"

        fun start(context: Context, serverUrl: String, token: String) {
            val intent = Intent(context, MessageService::class.java).apply {
                action = ACTION_START
                putExtra(EXTRA_SERVER_URL, serverUrl)
                putExtra(EXTRA_TOKEN, token)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            val intent = Intent(context, MessageService::class.java).apply {
                action = ACTION_STOP
            }
            context.startService(intent)
        }
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> {
                val serverUrl = intent.getStringExtra(EXTRA_SERVER_URL)
                val token = intent.getStringExtra(EXTRA_TOKEN)

                if (serverUrl != null && token != null && !isRunning) {
                    isRunning = true
                    startForeground(NOTIFICATION_ID, createNotification())
                    connectWebSocket(serverUrl, token)
                }
            }
            ACTION_STOP -> {
                stopService()
            }
        }
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder {
        return binder
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
        webSocketManager?.disconnect()
        isRunning = false
    }

    fun getWebSocketManager(): WebSocketManager? = webSocketManager

    private fun connectWebSocket(serverUrl: String, token: String) {
        scope.launch {
            try {
                _connectionState.value = ConnectionState.CONNECTING

                webSocketManager = WebSocketManager(serverUrl)
                webSocketManager?.connect(token)

                // Observe connection state
                webSocketManager?.connectionState?.collect { state ->
                    _connectionState.value = when (state) {
                        is WebSocketManager.ConnectionState.Connected -> ConnectionState.CONNECTED
                        is WebSocketManager.ConnectionState.Connecting -> ConnectionState.CONNECTING
                        is WebSocketManager.ConnectionState.Disconnected -> ConnectionState.DISCONNECTED
                    }
                    updateNotification()
                }
            } catch (e: Exception) {
                _connectionState.value = ConnectionState.DISCONNECTED
                // Retry connection after delay
                delay(5000)
                if (isRunning) {
                    connectWebSocket(serverUrl, token)
                }
            }
        }

        // Handle incoming messages
        scope.launch {
            webSocketManager?.incomingMessages?.collect { message ->
                handleIncomingMessage(message)
            }
        }
    }

    private fun handleIncomingMessage(message: Any) {
        // Increment unread count
        _unreadCount.value++

        // Show notification
        showMessageNotification(message)
    }

    private fun showMessageNotification(message: Any) {
        // In production, parse message and show appropriate notification
        val notificationManager = getSystemService(NotificationManager::class.java)

        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("New Message")
            .setContentText("You have a new message")
            .setSmallIcon(android.R.drawable.ic_dialog_email)
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .build()

        notificationManager.notify(System.currentTimeMillis().toInt(), notification)
    }

    private fun stopService() {
        isRunning = false
        webSocketManager?.disconnect()
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val serviceChannel = NotificationChannel(
                CHANNEL_ID,
                "Message Service",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Background message synchronization"
                setShowBadge(false)
            }

            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager.createNotificationChannel(serviceChannel)
        }
    }

    private fun createNotification(): Notification {
        val intent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val statusText = when (_connectionState.value) {
            ConnectionState.CONNECTED -> "Connected"
            ConnectionState.CONNECTING -> "Connecting..."
            ConnectionState.DISCONNECTED -> "Disconnected"
        }

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("PrivMsg")
            .setContentText(statusText)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    private fun updateNotification() {
        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.notify(NOTIFICATION_ID, createNotification())
    }

    fun clearUnreadCount() {
        _unreadCount.value = 0
    }

    inner class MessageBinder : Binder() {
        fun getService(): MessageService = this@MessageService
    }

    enum class ConnectionState {
        CONNECTED,
        CONNECTING,
        DISCONNECTED
    }
}
