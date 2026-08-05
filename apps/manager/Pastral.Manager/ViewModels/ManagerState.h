#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace Pastral::Manager::Presentation
{
    enum class ConnectionState
    {
        Loading,
        Connected,
        Disconnected,
        CapturePaused,
        ProtocolMismatch,
        Error,
    };

    enum class ManagerStatusCode
    {
        Loading,
        Connected,
        ConnectedFirstPage,
        ConnectedCurrentPage,
        Disconnected,
        Timeout,
        ProtocolMismatch,
        AuthenticationFailed,
        Unhealthy,
        InvalidConfiguration,
        AbiMismatch,
        HistoryBridgeUnavailable,
        HistoryChanged,
        InternalError,
        Synthetic,
    };

    struct ClipPreviewData
    {
        std::wstring id;
        std::wstring safePreview;
        std::wstring source;
        std::int64_t observedAtUnixMicros{};
        std::wstring typeLabel;
        std::wstring profile;
        std::wstring representationSummary;
        bool pinned{ false };
        bool unavailable{ false };
        bool previewTruncated{ false };
    };

    struct ManagerSnapshot
    {
        ConnectionState connection{ ConnectionState::Disconnected };
        ManagerStatusCode statusCode{ ManagerStatusCode::Disconnected };
        std::uint32_t storageSchemaVersion{};
        std::vector<ClipPreviewData> clips;
        std::wstring query;
        bool hasMore{ false };
        bool synthetic{ false };
    };
}
