package com.privmsg.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.privmsg.app.ui.theme.*

data class Message(
    val id: String,
    val content: String,
    val timestamp: String,
    val isOutgoing: Boolean,
    val status: MessageStatus = MessageStatus.SENT
)

enum class MessageStatus {
    SENDING, SENT, DELIVERED, READ
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatScreen(
    userId: String,
    onBack: () -> Unit,
    onCallClick: (Boolean) -> Unit
) {
    var messageText by remember { mutableStateOf("") }
    val listState = rememberLazyListState()

    // Mock data
    val userName = remember { "Alice" }
    val isOnline = remember { true }
    val messages = remember {
        mutableStateListOf(
            Message("1", "Hey!", "12:30", false),
            Message("2", "Hi! How are you?", "12:31", true, MessageStatus.READ),
            Message("3", "I'm good, thanks! How about you?", "12:32", false),
            Message("4", "Great! Just working on some stuff.", "12:33", true, MessageStatus.READ),
            Message("5", "Nice! What are you working on?", "12:34", false)
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Surface(
                            modifier = Modifier.size(40.dp),
                            shape = CircleShape,
                            color = Color(0xFF5856D6)
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                Text(
                                    text = userName.first().toString(),
                                    style = MaterialTheme.typography.titleMedium,
                                    color = Color.White
                                )
                            }
                        }
                        Spacer(modifier = Modifier.width(12.dp))
                        Column {
                            Text(
                                text = userName,
                                style = MaterialTheme.typography.titleMedium
                            )
                            Text(
                                text = if (isOnline) "online" else "last seen recently",
                                style = MaterialTheme.typography.bodySmall,
                                color = if (isOnline) Green else MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                },
                actions = {
                    IconButton(onClick = { onCallClick(false) }) {
                        Icon(Icons.Filled.Call, contentDescription = "Voice Call")
                    }
                    IconButton(onClick = { onCallClick(true) }) {
                        Icon(Icons.Filled.Videocam, contentDescription = "Video Call")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background
                )
            )
        },
        bottomBar = {
            ChatInput(
                value = messageText,
                onValueChange = { messageText = it },
                onSend = {
                    if (messageText.isNotBlank()) {
                        messages.add(
                            Message(
                                id = System.currentTimeMillis().toString(),
                                content = messageText,
                                timestamp = "Now",
                                isOutgoing = true,
                                status = MessageStatus.SENDING
                            )
                        )
                        messageText = ""
                    }
                },
                onAttach = { /* TODO */ },
                onVoice = { /* TODO */ }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 12.dp),
            state = listState,
            reverseLayout = false,
            verticalArrangement = Arrangement.spacedBy(4.dp),
            contentPadding = PaddingValues(vertical = 8.dp)
        ) {
            items(messages) { message ->
                MessageBubble(message = message)
            }
        }
    }
}

@Composable
private fun MessageBubble(message: Message) {
    val isOutgoing = message.isOutgoing

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isOutgoing) Arrangement.End else Arrangement.Start
    ) {
        Surface(
            shape = RoundedCornerShape(
                topStart = 18.dp,
                topEnd = 18.dp,
                bottomStart = if (isOutgoing) 18.dp else 4.dp,
                bottomEnd = if (isOutgoing) 4.dp else 18.dp
            ),
            color = if (isOutgoing) OutgoingBubble else MaterialTheme.colorScheme.surfaceVariant,
            modifier = Modifier.widthIn(max = 280.dp)
        ) {
            Column(
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp)
            ) {
                Text(
                    text = message.content,
                    style = MaterialTheme.typography.bodyLarge,
                    color = if (isOutgoing) Color.White else MaterialTheme.colorScheme.onSurface
                )

                Spacer(modifier = Modifier.height(2.dp))

                Row(
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.align(Alignment.End)
                ) {
                    Text(
                        text = message.timestamp,
                        style = MaterialTheme.typography.labelSmall,
                        color = if (isOutgoing)
                            Color.White.copy(alpha = 0.7f)
                        else
                            MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    if (isOutgoing) {
                        Spacer(modifier = Modifier.width(4.dp))
                        Icon(
                            imageVector = when (message.status) {
                                MessageStatus.SENDING -> Icons.Filled.Schedule
                                MessageStatus.SENT -> Icons.Filled.Check
                                MessageStatus.DELIVERED -> Icons.Filled.DoneAll
                                MessageStatus.READ -> Icons.Filled.DoneAll
                            },
                            contentDescription = null,
                            modifier = Modifier.size(14.dp),
                            tint = if (message.status == MessageStatus.READ)
                                Color.White
                            else
                                Color.White.copy(alpha = 0.7f)
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ChatInput(
    value: String,
    onValueChange: (String) -> Unit,
    onSend: () -> Unit,
    onAttach: () -> Unit,
    onVoice: () -> Unit
) {
    Surface(
        color = MaterialTheme.colorScheme.background,
        shadowElevation = 8.dp
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 8.dp, vertical = 8.dp),
            verticalAlignment = Alignment.Bottom
        ) {
            IconButton(onClick = onAttach) {
                Icon(
                    Icons.Filled.AttachFile,
                    contentDescription = "Attach",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Surface(
                modifier = Modifier.weight(1f),
                shape = RoundedCornerShape(20.dp),
                color = MaterialTheme.colorScheme.surfaceVariant
            ) {
                TextField(
                    value = value,
                    onValueChange = onValueChange,
                    placeholder = { Text("Message") },
                    colors = TextFieldDefaults.colors(
                        unfocusedContainerColor = Color.Transparent,
                        focusedContainerColor = Color.Transparent,
                        unfocusedIndicatorColor = Color.Transparent,
                        focusedIndicatorColor = Color.Transparent
                    ),
                    modifier = Modifier.fillMaxWidth(),
                    maxLines = 5
                )
            }

            Spacer(modifier = Modifier.width(4.dp))

            if (value.isBlank()) {
                IconButton(
                    onClick = onVoice,
                    modifier = Modifier
                        .size(44.dp)
                        .background(PrimaryBlue, CircleShape)
                ) {
                    Icon(
                        Icons.Filled.Mic,
                        contentDescription = "Voice Message",
                        tint = Color.White
                    )
                }
            } else {
                IconButton(
                    onClick = onSend,
                    modifier = Modifier
                        .size(44.dp)
                        .background(PrimaryBlue, CircleShape)
                ) {
                    Icon(
                        Icons.Filled.Send,
                        contentDescription = "Send",
                        tint = Color.White
                    )
                }
            }
        }
    }
}
