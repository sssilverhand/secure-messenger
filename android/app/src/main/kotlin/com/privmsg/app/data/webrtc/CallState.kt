package com.privmsg.app.data.webrtc

/**
 * Represents the current state of a call
 */
sealed class CallState {
    object Idle : CallState()
    data class Incoming(val peerId: String, val isVideo: Boolean) : CallState()
    data class Outgoing(val peerId: String, val isVideo: Boolean) : CallState()
    data class Connected(val peerId: String, val isVideo: Boolean, val duration: Long = 0) : CallState()
    data class Ended(val reason: String) : CallState()
}
