package com.privmsg.app.ui.components

import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.unit.dp
import kotlin.math.sin

/**
 * Voice message bubble for chat
 */
@Composable
fun VoiceMessageBubble(
    durationMs: Long,
    isPlaying: Boolean,
    progress: Float,
    isOutgoing: Boolean,
    onPlayPause: () -> Unit,
    onSeek: (Float) -> Unit,
    modifier: Modifier = Modifier
) {
    val backgroundColor = if (isOutgoing) {
        Color(0xFF007AFF)
    } else {
        MaterialTheme.colorScheme.surfaceVariant
    }

    val contentColor = if (isOutgoing) Color.White else MaterialTheme.colorScheme.onSurface

    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(18.dp),
        color = backgroundColor
    ) {
        Row(
            modifier = Modifier
                .padding(8.dp)
                .widthIn(min = 180.dp, max = 260.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Play/Pause button
            IconButton(
                onClick = onPlayPause,
                modifier = Modifier
                    .size(40.dp)
                    .background(
                        color = contentColor.copy(alpha = 0.2f),
                        shape = CircleShape
                    )
            ) {
                Icon(
                    imageVector = if (isPlaying) Icons.Filled.Pause else Icons.Filled.PlayArrow,
                    contentDescription = if (isPlaying) "Pause" else "Play",
                    tint = contentColor
                )
            }

            Spacer(modifier = Modifier.width(8.dp))

            Column(modifier = Modifier.weight(1f)) {
                // Waveform
                VoiceWaveform(
                    progress = progress,
                    isPlaying = isPlaying,
                    color = contentColor,
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(24.dp)
                        .clickable { /* handle seek */ }
                )

                Spacer(modifier = Modifier.height(4.dp))

                // Duration
                Text(
                    text = formatDuration(durationMs),
                    style = MaterialTheme.typography.labelSmall,
                    color = contentColor.copy(alpha = 0.7f)
                )
            }
        }
    }
}

/**
 * Animated waveform visualization
 */
@Composable
fun VoiceWaveform(
    progress: Float,
    isPlaying: Boolean,
    color: Color,
    modifier: Modifier = Modifier
) {
    val infiniteTransition = rememberInfiniteTransition(label = "waveform")
    val animatedPhase by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 2f * Math.PI.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(1000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "phase"
    )

    Canvas(modifier = modifier) {
        val width = size.width
        val height = size.height
        val centerY = height / 2

        val barCount = 30
        val barWidth = 3.dp.toPx()
        val barSpacing = (width - barCount * barWidth) / (barCount - 1)

        for (i in 0 until barCount) {
            val x = i * (barWidth + barSpacing) + barWidth / 2
            val normalizedX = i.toFloat() / barCount

            // Generate pseudo-random but consistent heights based on position
            val baseHeight = 0.3f + 0.7f * sin((normalizedX * 10 + i * 0.5f).toDouble()).toFloat().coerceIn(0f, 1f)

            val barHeight = if (isPlaying && normalizedX <= progress) {
                // Animated bars for played portion
                val animOffset = if (normalizedX <= progress) {
                    sin((animatedPhase + i * 0.3f).toDouble()).toFloat() * 0.2f
                } else 0f
                (baseHeight + animOffset).coerceIn(0.2f, 1f) * height * 0.8f
            } else {
                // Static bars
                baseHeight * height * 0.6f
            }

            val barColor = if (normalizedX <= progress) {
                color
            } else {
                color.copy(alpha = 0.4f)
            }

            drawLine(
                color = barColor,
                start = Offset(x, centerY - barHeight / 2),
                end = Offset(x, centerY + barHeight / 2),
                strokeWidth = barWidth,
                cap = StrokeCap.Round
            )
        }
    }
}

/**
 * Voice recording button (hold to record)
 */
@Composable
fun VoiceRecordButton(
    isRecording: Boolean,
    recordingDuration: Long,
    onStartRecording: () -> Unit,
    onStopRecording: () -> Unit,
    onCancelRecording: () -> Unit,
    modifier: Modifier = Modifier
) {
    val pulseAnimation = rememberInfiniteTransition(label = "pulse")
    val scale by pulseAnimation.animateFloat(
        initialValue = 1f,
        targetValue = 1.2f,
        animationSpec = infiniteRepeatable(
            animation = tween(500),
            repeatMode = RepeatMode.Reverse
        ),
        label = "scale"
    )

    if (isRecording) {
        Row(
            modifier = modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Cancel button
            IconButton(onClick = onCancelRecording) {
                Icon(
                    Icons.Filled.Delete,
                    contentDescription = "Cancel",
                    tint = MaterialTheme.colorScheme.error
                )
            }

            Spacer(modifier = Modifier.weight(1f))

            // Recording indicator
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    modifier = Modifier
                        .size((12 * scale).dp)
                        .background(Color.Red, CircleShape)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = formatDuration(recordingDuration),
                    style = MaterialTheme.typography.bodyMedium
                )
            }

            Spacer(modifier = Modifier.weight(1f))

            // Send button
            IconButton(
                onClick = onStopRecording,
                modifier = Modifier
                    .size(48.dp)
                    .background(Color(0xFF007AFF), CircleShape)
            ) {
                Icon(
                    Icons.Filled.Send,
                    contentDescription = "Send",
                    tint = Color.White
                )
            }
        }
    } else {
        IconButton(
            onClick = onStartRecording,
            modifier = modifier
                .size(44.dp)
                .background(Color(0xFF007AFF), CircleShape)
        ) {
            Icon(
                Icons.Filled.Mic,
                contentDescription = "Record voice message",
                tint = Color.White
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
