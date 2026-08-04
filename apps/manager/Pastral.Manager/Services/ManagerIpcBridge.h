#pragma once

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

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
        InsufficientBuffer = 9,
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

    enum class ManagerIpcBridgeClipKind : std::uint32_t
    {
        Unavailable = 0,
        Text = 1,
    };

    struct ManagerIpcBridgeClip
    {
        std::array<std::uint8_t, 16> eventId{};
        std::uint64_t captureOrder{};
        std::int64_t observedAtUnixMicros{};
        ManagerIpcBridgeClipKind kind{ ManagerIpcBridgeClipKind::Unavailable };
        std::wstring preview;
        std::optional<std::wstring> sourceLabel;
        bool pinned{};
        bool unavailable{};
        bool previewTruncated{};
    };

    struct ManagerIpcBridgePage
    {
        ManagerIpcBridgeStatus status{ ManagerIpcBridgeStatus::InternalError };
        std::vector<ManagerIpcBridgeClip> items;
        bool hasMore{};
        std::uint32_t serverProcessId{};
        std::uint32_t sessionId{};
        std::uint64_t connectMicroseconds{};
        std::uint64_t handshakeMicroseconds{};
        std::uint64_t requestMicroseconds{};
    };

    class ManagerIpcBridge final
    {
    public:
        ManagerIpcBridge() = delete;

        [[nodiscard]] static bool IsAvailable() noexcept;
        [[nodiscard]] static bool IsReadAvailable() noexcept;
        [[nodiscard]] static ManagerIpcBridgeHealth QueryHealth(
            std::wstring const& dataRoot,
            std::uint32_t timeoutMilliseconds) noexcept;
        [[nodiscard]] static ManagerIpcBridgePage QueryHistory(
            std::wstring const& dataRoot,
            std::uint32_t timeoutMilliseconds,
            std::uint32_t limit,
            std::optional<std::uint64_t> beforeCaptureOrder = std::nullopt) noexcept;
        [[nodiscard]] static ManagerIpcBridgePage QuerySearch(
            std::wstring const& dataRoot,
            std::uint32_t timeoutMilliseconds,
            std::wstring const& query,
            std::uint32_t limit) noexcept;
    };
}
