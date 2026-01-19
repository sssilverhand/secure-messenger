package com.privmsg.app.ui.screens

import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.privmsg.app.ui.theme.Green
import com.privmsg.app.ui.theme.PrimaryBlue
import com.privmsg.app.ui.theme.Red
import kotlinx.coroutines.delay

enum class CallState {
    RINGING, CONNECTING, CONNECTED, ENDED
}

@Composable
fun CallScreen(
    userId: String,
    isVideo: Boolean,
    isIncoming: Boolean,
    onEnd: () -> Unit
) {
    var callState by remember { mutableStateOf(if (isIncoming) CallState.RINGING else CallState.CONNECTING) }
    var isMuted by remember { mutableStateOf(false) }
    var isSpeakerOn by remember { mutableStateOf(false) }
    var isVideoEnabled by remember { mutableStateOf(isVideo) }
    var callDuration by remember { mutableStateOf(0) }

    // Simulate call connection
    LaunchedEffect(callState) {
        if (callState == CallState.CONNECTING) {
            delay(2000)
            callState = CallState.CONNECTED
        }
    }

    // Call duration timer
    LaunchedEffect(callState) {
        if (callState == CallState.CONNECTED) {
            while (true) {
                delay(1000)
                callDuration++
            }
        }
    }

    // Pulsing animation for ringing/connecting
    val infiniteTransition = rememberInfiniteTransition(label = "pulse")
    val pulseScale by infiniteTransition.animateFloat(
        initialValue = 1f,
        targetValue = 1.2f,
        animationSpec = infiniteRepeatable(
            animation = tween(1000),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulseScale"
    )

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(
                Brush.verticalGradient(
                    colors = listOf(
                        Color(0xFF1C1C1E),
                        Color(0xFF000000)
                    )
                )
            )
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Spacer(modifier = Modifier.height(60.dp))

            // Avatar with pulse animation
            Box(contentAlignment = Alignment.Center) {
                if (callState == CallState.RINGING || callState == CallState.CONNECTING) {
                    Surface(
                        modifier = Modifier
                            .size(140.dp)
                            .scale(pulseScale),
                        shape = CircleShape,
                        color = PrimaryBlue.copy(alpha = 0.2f)
                    ) {}
                }

                Surface(
                    modifier = Modifier.size(120.dp),
                    shape = CircleShape,
                    color = PrimaryBlue
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(
                            text = userId.first().uppercase(),
                            fontSize = 48.sp,
                            color = Color.White
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            // Name
            Text(
                text = userId,
                style = MaterialTheme.typography.headlineMedium,
                color = Color.White
            )

            Spacer(modifier = Modifier.height(8.dp))

            // Status
            Text(
                text = when (callState) {
                    CallState.RINGING -> if (isIncoming) "Incoming ${if (isVideo) "video" else "voice"} call..." else "Calling..."
                    CallState.CONNECTING -> "Connecting..."
                    CallState.CONNECTED -> formatDuration(callDuration)
                    CallState.ENDED -> "Call ended"
                },
                style = MaterialTheme.typography.bodyLarge,
                color = Color.White.copy(alpha = 0.7f)
            )

            Spacer(modifier = Modifier.weight(1f))

            // Video preview placeholder (when video call)
            if (isVideo && callState == CallState.CONNECTED) {
                Surface(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(200.dp),
                    shape = RoundedCornerShape(16.dp),
                    color = Color(0xFF2C2C2E)
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        if (isVideoEnabled) {
                            Text(
                                text = "Video Preview",
                                color = Color.White.copy(alpha = 0.5f)
                            )
                        } else {
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally
                            ) {
                                Icon(
                                    Icons.Filled.VideocamOff,
                                    contentDescription = null,
                                    tint = Color.White.copy(alpha = 0.5f),
                                    modifier = Modifier.size(48.dp)
                                )
                                Spacer(modifier = Modifier.height(8.dp))
                                Text(
                                    text = "Camera Off",
                                    color = Color.White.copy(alpha = 0.5f)
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(32.dp))
            }

            // Call controls
            if (callState == CallState.RINGING && isIncoming) {
                // Incoming call: Accept / Decline
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceEvenly
                ) {
                    CallButton(
                        icon = Icons.Filled.CallEnd,
                        label = "Decline",
                        backgroundColor = Red,
                        onClick = onEnd
                    )

                    CallButton(
                        icon = if (isVideo) Icons.Filled.Videocam else Icons.Filled.Call,
                        label = "Accept",
                        backgroundColor = Green,
                        onClick = { callState = CallState.CONNECTING }
                    )
                }
            } else if (callState != CallState.ENDED) {
                // Active call controls
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceEvenly
                ) {
                    CallControlButton(
                        icon = if (isMuted) Icons.Filled.MicOff else Icons.Filled.Mic,
                        label = "Mute",
                        isActive = isMuted,
                        onClick = { isMuted = !isMuted }
                    )

                    if (isVideo) {
                        CallControlButton(
                            icon = if (isVideoEnabled) Icons.Filled.Videocam else Icons.Filled.VideocamOff,
                            label = "Video",
                            isActive = !isVideoEnabled,
                            onClick = { isVideoEnabled = !isVideoEnabled }
                        )
                    }

                    CallControlButton(
                        icon = if (isSpeakerOn) Icons.Filled.VolumeUp else Icons.Filled.VolumeDown,
                        label = "Speaker",
                        isActive = isSpeakerOn,
                        onClick = { isSpeakerOn = !isSpeakerOn }
                    )
                }

                Spacer(modifier = Modifier.height(32.dp))

                // End call button
                CallButton(
                    icon = Icons.Filled.CallEnd,
                    label = "End",
                    backgroundColor = Red,
                    onClick = onEnd,
                    size = 72.dp
                )
            }

            Spacer(modifier = Modifier.height(40.dp))
        }
    }
}

@Composable
private fun CallButton(
    icon: ImageVector,
    label: String,
    backgroundColor: Color,
    onClick: () -> Unit,
    size: androidx.compose.ui.unit.Dp = 64.dp
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Surface(
            modifier = Modifier.size(size),
            shape = CircleShape,
            color = backgroundColor,
            onClick = onClick
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    imageVector = icon,
                    contentDescription = label,
                    tint = Color.White,
                    modifier = Modifier.size(size / 2)
                )
            }
        }

        Spacer(modifier = Modifier.height(8.dp))

        Text(
            text = label,
            style = MaterialTheme.typography.bodySmall,
            color = Color.White.copy(alpha = 0.7f)
        )
    }
}

@Composable
private fun CallControlButton(
    icon: ImageVector,
    label: String,
    isActive: Boolean,
    onClick: () -> Unit
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Surface(
            modifier = Modifier.size(56.dp),
            shape = CircleShape,
            color = if (isActive) Color.White else Color.White.copy(alpha = 0.2f),
            onClick = onClick
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    imageVector = icon,
                    contentDescription = label,
                    tint = if (isActive) Color.Black else Color.White,
                    modifier = Modifier.size(24.dp)
                )
            }
        }

        Spacer(modifier = Modifier.height(8.dp))

        Text(
            text = label,
            style = MaterialTheme.typography.bodySmall,
            color = Color.White.copy(alpha = 0.7f)
        )
    }
}

private fun formatDuration(seconds: Int): String {
    val minutes = seconds / 60
    val secs = seconds % 60
    return "%02d:%02d".format(minutes, secs)
}
