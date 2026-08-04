#pragma once

#include <cstdint>
#include <string>

namespace Pastral::Manager::Services
{
    enum class ManagerIpcBridgeStatus : std::uint32_t
    {
        Connected = 0,
        Disconnected = 1,
        Timeout = 2,
        ProtocolMismatch = 3,
        AuthenticationFailed = 4,
        Unhealthy = 5,
        InvalidArgument = 6,
        InternalError = 7,
        AbiMismatch = 8,
    };

    struct ManagerIpcBridgeHealth
    {
        ManagerIpcBridgeStatus status{ ManagerIpcBridgeStatus::InternalError };
        std::uint32_t storageSchemaVersion{};
        bool captureEnabled{};
        bool privacyPolicyOk{};
        bool storageIntegrityOk{};
        std::uint32_t serverProcessId{};
        std::uint32_t sessionId{};
        std::uint64_t connectMicroseconds{};
        std::uint64_t handshakeMicroseconds{};
        std::uint64_t healthMicroseconds{};
    };

    class ManagerIpcBridge final
    {
    public:
        ManagerIpcBridge() = delete;

        [[nodiscard]] static bool IsAvailable() noexcept;
        [[nodiscard]] static ManagerIpcBridgeHealth QueryHealth(
            std::wstring const& dataRoot,
            std::uint32_t timeoutMilliseconds) noexcept;
    };
}
