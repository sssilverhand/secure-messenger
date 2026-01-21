package com.privmsg.app.data.webrtc

import android.content.Context
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.webrtc.*

/**
 * Manages WebRTC calls (voice and video)
 */
class CallManager(private val context: Context) {

    private var peerConnectionFactory: PeerConnectionFactory? = null
    private var peerConnection: PeerConnection? = null
    private var localAudioTrack: AudioTrack? = null
    private var localVideoTrack: VideoTrack? = null
    private var videoCapturer: VideoCapturer? = null

    private val _callState = MutableStateFlow<CallState>(CallState.Idle)
    val callState: StateFlow<CallState> = _callState

    private var currentPeerId: String? = null
    private var isVideoCall: Boolean = false

    init {
        initializePeerConnectionFactory()
    }

    private fun initializePeerConnectionFactory() {
        val options = PeerConnectionFactory.InitializationOptions.builder(context)
            .setEnableInternalTracer(true)
            .createInitializationOptions()
        PeerConnectionFactory.initialize(options)

        val encoderFactory = DefaultVideoEncoderFactory(
            EglBase.create().eglBaseContext,
            true,
            true
        )
        val decoderFactory = DefaultVideoDecoderFactory(EglBase.create().eglBaseContext)

        peerConnectionFactory = PeerConnectionFactory.builder()
            .setVideoEncoderFactory(encoderFactory)
            .setVideoDecoderFactory(decoderFactory)
            .createPeerConnectionFactory()
    }

    /**
     * Start an outgoing call
     */
    suspend fun startCall(
        peerId: String,
        isVideo: Boolean,
        localVideoSink: VideoSink?,
        remoteVideoSink: VideoSink?
    ) {
        currentPeerId = peerId
        isVideoCall = isVideo
        _callState.value = CallState.Outgoing(peerId, isVideo)

        createPeerConnection()
        createLocalTracks(isVideo, localVideoSink)

        // Create and send offer
        val constraints = MediaConstraints().apply {
            mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveAudio", "true"))
            if (isVideo) {
                mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveVideo", "true"))
            }
        }

        peerConnection?.createOffer(object : SdpObserver {
            override fun onCreateSuccess(sdp: SessionDescription) {
                peerConnection?.setLocalDescription(object : SdpObserver {
                    override fun onCreateSuccess(sdp: SessionDescription?) {}
                    override fun onSetSuccess() {
                        // TODO: Send offer SDP to signaling server
                    }
                    override fun onCreateFailure(error: String?) {}
                    override fun onSetFailure(error: String?) {}
                }, sdp)
            }
            override fun onSetSuccess() {}
            override fun onCreateFailure(error: String?) {}
            override fun onSetFailure(error: String?) {}
        }, constraints)
    }

    /**
     * Accept an incoming call
     */
    fun acceptCall(peerId: String) {
        _callState.value = CallState.Connected(peerId, isVideoCall)
        // TODO: Create answer and send to peer
    }

    /**
     * Reject an incoming call
     */
    fun rejectCall(peerId: String) {
        _callState.value = CallState.Ended("Rejected")
        // TODO: Send rejection to peer
    }

    /**
     * End the current call
     */
    fun endCall() {
        peerConnection?.close()
        peerConnection = null
        localAudioTrack = null
        localVideoTrack = null
        videoCapturer?.stopCapture()
        videoCapturer = null
        currentPeerId = null
        _callState.value = CallState.Ended("Call ended")
    }

    /**
     * Toggle mute state
     */
    fun toggleMute(): Boolean {
        localAudioTrack?.let {
            it.setEnabled(!it.enabled())
            return !it.enabled()
        }
        return false
    }

    /**
     * Toggle video state
     */
    fun toggleVideo(): Boolean {
        localVideoTrack?.let {
            it.setEnabled(!it.enabled())
            return !it.enabled()
        }
        return false
    }

    /**
     * Toggle speaker
     */
    fun toggleSpeaker(): Boolean {
        // Implemented in CallService via AudioManager
        return false
    }

    /**
     * Release all resources
     */
    fun release() {
        endCall()
        peerConnectionFactory?.dispose()
        peerConnectionFactory = null
    }

    private fun createPeerConnection() {
        val iceServers = listOf(
            PeerConnection.IceServer.builder("stun:stun.l.google.com:19302").createIceServer()
        )

        val rtcConfig = PeerConnection.RTCConfiguration(iceServers).apply {
            sdpSemantics = PeerConnection.SdpSemantics.UNIFIED_PLAN
        }

        peerConnection = peerConnectionFactory?.createPeerConnection(
            rtcConfig,
            object : PeerConnection.Observer {
                override fun onSignalingChange(state: PeerConnection.SignalingState?) {}
                override fun onIceConnectionChange(state: PeerConnection.IceConnectionState?) {
                    when (state) {
                        PeerConnection.IceConnectionState.CONNECTED -> {
                            currentPeerId?.let {
                                _callState.value = CallState.Connected(it, isVideoCall)
                            }
                        }
                        PeerConnection.IceConnectionState.DISCONNECTED,
                        PeerConnection.IceConnectionState.FAILED -> {
                            _callState.value = CallState.Ended("Connection lost")
                        }
                        else -> {}
                    }
                }
                override fun onIceConnectionReceivingChange(receiving: Boolean) {}
                override fun onIceGatheringChange(state: PeerConnection.IceGatheringState?) {}
                override fun onIceCandidate(candidate: IceCandidate?) {
                    // TODO: Send ICE candidate to signaling server
                }
                override fun onIceCandidatesRemoved(candidates: Array<out IceCandidate>?) {}
                override fun onAddStream(stream: MediaStream?) {}
                override fun onRemoveStream(stream: MediaStream?) {}
                override fun onDataChannel(channel: DataChannel?) {}
                override fun onRenegotiationNeeded() {}
                override fun onAddTrack(receiver: RtpReceiver?, streams: Array<out MediaStream>?) {}
            }
        )
    }

    private fun createLocalTracks(isVideo: Boolean, localVideoSink: VideoSink?) {
        // Create audio track
        val audioConstraints = MediaConstraints()
        val audioSource = peerConnectionFactory?.createAudioSource(audioConstraints)
        localAudioTrack = peerConnectionFactory?.createAudioTrack("audio0", audioSource)
        localAudioTrack?.let { peerConnection?.addTrack(it) }

        // Create video track if needed
        if (isVideo && localVideoSink != null) {
            videoCapturer = createCameraCapturer()
            videoCapturer?.let { capturer ->
                val surfaceTextureHelper = SurfaceTextureHelper.create(
                    "CaptureThread",
                    EglBase.create().eglBaseContext
                )
                val videoSource = peerConnectionFactory?.createVideoSource(capturer.isScreencast)
                capturer.initialize(surfaceTextureHelper, context, videoSource?.capturerObserver)
                capturer.startCapture(1280, 720, 30)

                localVideoTrack = peerConnectionFactory?.createVideoTrack("video0", videoSource)
                localVideoTrack?.addSink(localVideoSink)
                localVideoTrack?.let { peerConnection?.addTrack(it) }
            }
        }
    }

    private fun createCameraCapturer(): VideoCapturer? {
        val enumerator = Camera2Enumerator(context)
        val deviceNames = enumerator.deviceNames

        // Try front camera first
        for (deviceName in deviceNames) {
            if (enumerator.isFrontFacing(deviceName)) {
                return enumerator.createCapturer(deviceName, null)
            }
        }

        // Fall back to any camera
        for (deviceName in deviceNames) {
            return enumerator.createCapturer(deviceName, null)
        }

        return null
    }
}
