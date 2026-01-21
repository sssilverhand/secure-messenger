package com.privmsg.app.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Binder
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.privmsg.app.MainActivity
import com.privmsg.app.data.webrtc.CallManager
import com.privmsg.app.data.webrtc.CallState
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * Foreground service for handling voice and video calls.
 * Keeps the call alive even when app is in background.
 */
class CallService : Service() {

    private val binder = CallBinder()
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private var callManager: CallManager? = null
    private var audioManager: AudioManager? = null
    private var audioFocusRequest: AudioFocusRequest? = null

    private val _callState = MutableStateFlow<CallState>(CallState.Idle)
    val callState: StateFlow<CallState> = _callState

    companion object {
        const val CHANNEL_ID = "privmsg_call_channel"
        const val NOTIFICATION_ID = 1001

        const val ACTION_START_CALL = "com.privmsg.app.START_CALL"
        const val ACTION_ACCEPT_CALL = "com.privmsg.app.ACCEPT_CALL"
        const val ACTION_REJECT_CALL = "com.privmsg.app.REJECT_CALL"
        const val ACTION_END_CALL = "com.privmsg.app.END_CALL"

        const val EXTRA_PEER_ID = "peer_id"
        const val EXTRA_IS_VIDEO = "is_video"
        const val EXTRA_IS_INCOMING = "is_incoming"

        fun startCall(context: Context, peerId: String, isVideo: Boolean) {
            val intent = Intent(context, CallService::class.java).apply {
                action = ACTION_START_CALL
                putExtra(EXTRA_PEER_ID, peerId)
                putExtra(EXTRA_IS_VIDEO, isVideo)
                putExtra(EXTRA_IS_INCOMING, false)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun handleIncomingCall(context: Context, peerId: String, isVideo: Boolean) {
            val intent = Intent(context, CallService::class.java).apply {
                action = ACTION_START_CALL
                putExtra(EXTRA_PEER_ID, peerId)
                putExtra(EXTRA_IS_VIDEO, isVideo)
                putExtra(EXTRA_IS_INCOMING, true)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        callManager = CallManager(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START_CALL -> {
                val peerId = intent.getStringExtra(EXTRA_PEER_ID) ?: return START_NOT_STICKY
                val isVideo = intent.getBooleanExtra(EXTRA_IS_VIDEO, false)
                val isIncoming = intent.getBooleanExtra(EXTRA_IS_INCOMING, false)

                startForeground(NOTIFICATION_ID, createCallNotification(peerId, isIncoming, isVideo))
                requestAudioFocus()

                if (isIncoming) {
                    _callState.value = CallState.Incoming(peerId, isVideo)
                } else {
                    scope.launch {
                        callManager?.startCall(peerId, isVideo, null, null)
                    }
                }
            }
            ACTION_ACCEPT_CALL -> {
                val currentState = _callState.value
                if (currentState is CallState.Incoming) {
                    callManager?.acceptCall(currentState.peerId)
                }
            }
            ACTION_REJECT_CALL -> {
                val currentState = _callState.value
                if (currentState is CallState.Incoming) {
                    callManager?.rejectCall(currentState.peerId)
                }
                endCall()
            }
            ACTION_END_CALL -> {
                endCall()
            }
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder {
        return binder
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
        callManager?.release()
        abandonAudioFocus()
    }

    fun getCallManager(): CallManager? = callManager

    fun endCall() {
        callManager?.endCall()
        abandonAudioFocus()
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Calls",
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Ongoing call notifications"
                setSound(null, null)
            }
            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager.createNotificationChannel(channel)
        }
    }

    private fun createCallNotification(peerId: String, isIncoming: Boolean, isVideo: Boolean): Notification {
        val contentIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val callType = if (isVideo) "Video call" else "Voice call"
        val title = if (isIncoming) "Incoming $callType" else callType
        val text = if (isIncoming) "From $peerId" else "With $peerId"

        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_menu_call)
            .setContentIntent(contentIntent)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_CALL)

        if (isIncoming) {
            // Accept action
            val acceptIntent = PendingIntent.getService(
                this,
                1,
                Intent(this, CallService::class.java).apply {
                    action = ACTION_ACCEPT_CALL
                },
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
            builder.addAction(android.R.drawable.ic_menu_call, "Accept", acceptIntent)

            // Reject action
            val rejectIntent = PendingIntent.getService(
                this,
                2,
                Intent(this, CallService::class.java).apply {
                    action = ACTION_REJECT_CALL
                },
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
            builder.addAction(android.R.drawable.ic_menu_close_clear_cancel, "Reject", rejectIntent)
        } else {
            // End call action
            val endIntent = PendingIntent.getService(
                this,
                3,
                Intent(this, CallService::class.java).apply {
                    action = ACTION_END_CALL
                },
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
            builder.addAction(android.R.drawable.ic_menu_close_clear_cancel, "End", endIntent)
        }

        return builder.build()
    }

    private fun requestAudioFocus() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val audioAttributes = AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                .build()

            audioFocusRequest = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT)
                .setAudioAttributes(audioAttributes)
                .setAcceptsDelayedFocusGain(false)
                .setOnAudioFocusChangeListener { }
                .build()

            audioManager?.requestAudioFocus(audioFocusRequest!!)
        } else {
            @Suppress("DEPRECATION")
            audioManager?.requestAudioFocus(
                null,
                AudioManager.STREAM_VOICE_CALL,
                AudioManager.AUDIOFOCUS_GAIN_TRANSIENT
            )
        }

        // Set audio mode for voice call
        audioManager?.mode = AudioManager.MODE_IN_COMMUNICATION
    }

    private fun abandonAudioFocus() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            audioFocusRequest?.let {
                audioManager?.abandonAudioFocusRequest(it)
            }
        } else {
            @Suppress("DEPRECATION")
            audioManager?.abandonAudioFocus(null)
        }
        audioManager?.mode = AudioManager.MODE_NORMAL
    }

    inner class CallBinder : Binder() {
        fun getService(): CallService = this@CallService
    }
}
