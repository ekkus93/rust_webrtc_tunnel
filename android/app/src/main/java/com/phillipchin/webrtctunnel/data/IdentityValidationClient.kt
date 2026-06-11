package com.phillipchin.webrtctunnel.data

import com.phillipchin.webrtctunnel.RustTunnelBridge
import com.phillipchin.webrtctunnel.TunnelNativeBridge
import com.phillipchin.webrtctunnel.model.IdentityValidationResult
import com.phillipchin.webrtctunnel.model.ValidationResult

/**
 * Config/identity validation over the native bridge. Split from [TunnelRepository]
 * (which owns runtime lifecycle + status) so each stays a focused collaborator;
 * both share a single native bridge instance (see AppDependencies).
 */
class IdentityValidationClient(
    bridgeFactory: () -> TunnelNativeBridge = { RustTunnelBridge() },
) {
    constructor(bridge: TunnelNativeBridge) : this({ bridge })

    private val bridge: TunnelNativeBridge by lazy(bridgeFactory)

    fun validateConfig(configPath: String): ValidationResult = bridge.validateConfig(configPath)

    fun validateConfigWithIdentity(
        configPath: String,
        identityBytes: ByteArray,
    ): ValidationResult = bridge.validateConfigWithIdentity(configPath, identityBytes)

    fun validatePrivateIdentity(identityToml: String): IdentityValidationResult =
        bridge.validatePrivateIdentity(identityToml)

    fun validatePublicIdentity(line: String): IdentityValidationResult = bridge.validatePublicIdentity(line)

    fun generateIdentity(peerId: String): IdentityValidationResult = bridge.generateIdentity(peerId)
}
