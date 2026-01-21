package com.privmsg.app.ui.components

import android.view.ViewGroup
import androidx.annotation.OptIn
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.media3.common.MediaItem
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.ui.PlayerView

/**
 * Circular video message bubble (like Telegram video messages)
 */
@OptIn(UnstableApi::class)
@Composable
fun VideoMessageBubble(
    videoUrl: String,
    durationMs: Long,
    isOutgoing: Boolean,
    onLongPress: () -> Unit = {},
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    var isPlaying by remember { mutableStateOf(false) }
    var player by remember { mutableStateOf<ExoPlayer?>(null) }
    var currentPosition by remember { mutableStateOf(0L) }

    // Create ExoPlayer
    DisposableEffect(videoUrl) {
        val exoPlayer = ExoPlayer.Builder(context).build().apply {
            setMediaItem(MediaItem.fromUri(videoUrl))
            prepare()
            playWhenReady = false
        }
        player = exoPlayer

        onDispose {
            exoPlayer.release()
        }
    }

    // Video size
    val videoSize = 200.dp

    Box(
        modifier = modifier
            .size(videoSize)
            .clip(CircleShape)
            .border(
                width = 3.dp,
                brush = Brush.linearGradient(
                    colors = if (isOutgoing) {
                        listOf(Color(0xFF007AFF), Color(0xFF5856D6))
                    } else {
                        listOf(Color(0xFF34C759), Color(0xFF30D158))
                    }
                ),
                shape = CircleShape
            )
            .clickable {
                player?.let {
                    if (it.isPlaying) {
                        it.pause()
                        isPlaying = false
                    } else {
                        it.play()
                        isPlaying = true
                    }
                }
            }
    ) {
        // Video player
        player?.let { exoPlayer ->
            AndroidView(
                factory = { ctx ->
                    PlayerView(ctx).apply {
                        this.player = exoPlayer
                        useController = false
                        layoutParams = ViewGroup.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.MATCH_PARENT
                        )
                    }
                },
                modifier = Modifier.fillMaxSize()
            )
        }

        // Play button overlay (when not playing)
        if (!isPlaying) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color.Black.copy(alpha = 0.3f)),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    Icons.Filled.PlayArrow,
                    contentDescription = "Play",
                    tint = Color.White,
                    modifier = Modifier.size(48.dp)
                )
            }
        }

        // Duration badge
        Box(
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(8.dp)
                .background(
                    Color.Black.copy(alpha = 0.6f),
                    shape = MaterialTheme.shapes.small
                )
                .padding(horizontal = 6.dp, vertical = 2.dp)
        ) {
            Text(
                text = formatDuration(if (isPlaying) currentPosition else durationMs),
                style = MaterialTheme.typography.labelSmall,
                color = Color.White
            )
        }

        // Progress ring
        if (isPlaying) {
            CircularProgressIndicator(
                progress = (currentPosition.toFloat() / durationMs).coerceIn(0f, 1f),
                modifier = Modifier.fillMaxSize(),
                strokeWidth = 3.dp,
                color = Color.White.copy(alpha = 0.8f)
            )
        }
    }
}

/**
 * Video recording button with preview (circular)
 */
@Composable
fun VideoRecordButton(
    isRecording: Boolean,
    recordingDuration: Long,
    maxDuration: Long = 60_000L,
    onStartRecording: () -> Unit,
    onStopRecording: () -> Unit,
    onCancelRecording: () -> Unit,
    cameraPreview: @Composable () -> Unit,
    modifier: Modifier = Modifier
) {
    val progress = (recordingDuration.toFloat() / maxDuration).coerceIn(0f, 1f)

    val pulseAnimation = rememberInfiniteTransition(label = "pulse")
    val borderWidth by pulseAnimation.animateFloat(
        initialValue = 3f,
        targetValue = 5f,
        animationSpec = infiniteRepeatable(
            animation = tween(500),
            repeatMode = RepeatMode.Reverse
        ),
        label = "border"
    )

    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        if (isRecording) {
            // Recording preview
            Box(
                modifier = Modifier
                    .size(200.dp)
                    .clip(CircleShape)
                    .border(
                        width = borderWidth.dp,
                        color = Color.Red,
                        shape = CircleShape
                    )
            ) {
                cameraPreview()

                // Progress ring
                CircularProgressIndicator(
                    progress = progress,
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(borderWidth.dp),
                    strokeWidth = 4.dp,
                    color = Color.Red
                )

                // Duration
                Box(
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = 16.dp)
                        .background(
                            Color.Black.copy(alpha = 0.6f),
                            shape = MaterialTheme.shapes.small
                        )
                        .padding(horizontal = 8.dp, vertical = 4.dp)
                ) {
                    Text(
                        text = formatDuration(recordingDuration),
                        style = MaterialTheme.typography.bodyMedium,
                        color = Color.White
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            // Controls
            Row(
                horizontalArrangement = Arrangement.spacedBy(24.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Cancel
                IconButton(
                    onClick = onCancelRecording,
                    modifier = Modifier
                        .size(48.dp)
                        .background(Color.Gray.copy(alpha = 0.3f), CircleShape)
                ) {
                    Icon(
                        Icons.Filled.Close,
                        contentDescription = "Cancel",
                        tint = Color.White
                    )
                }

                // Stop/Send
                IconButton(
                    onClick = onStopRecording,
                    modifier = Modifier
                        .size(64.dp)
                        .background(Color.Red, CircleShape)
                ) {
                    Icon(
                        Icons.Filled.Stop,
                        contentDescription = "Stop",
                        tint = Color.White,
                        modifier = Modifier.size(32.dp)
                    )
                }

                // Switch camera
                IconButton(
                    onClick = { /* switch camera */ },
                    modifier = Modifier
                        .size(48.dp)
                        .background(Color.Gray.copy(alpha = 0.3f), CircleShape)
                ) {
                    Icon(
                        Icons.Filled.Cameraswitch,
                        contentDescription = "Switch camera",
                        tint = Color.White
                    )
                }
            }
        } else {
            // Start recording button
            IconButton(
                onClick = onStartRecording,
                modifier = Modifier
                    .size(64.dp)
                    .background(
                        Brush.linearGradient(
                            colors = listOf(Color(0xFF007AFF), Color(0xFF5856D6))
                        ),
                        CircleShape
                    )
            ) {
                Icon(
                    Icons.Filled.Videocam,
                    contentDescription = "Record video message",
                    tint = Color.White,
                    modifier = Modifier.size(32.dp)
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            Text(
                text = "Hold to record",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

/**
 * Format duration from milliseconds to mm:ss
 */
private fun formatDuration(durationMs: Long): String {
    val totalSeconds = durationMs / 1000
    val minutes = totalSeconds / 60
    val seconds = totalSeconds % 60
    return "%d:%02d".format(minutes, seconds)
}
