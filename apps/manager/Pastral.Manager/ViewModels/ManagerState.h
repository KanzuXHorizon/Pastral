#pragma once

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

    struct ClipPreviewData
    {
        std::wstring id;
        std::wstring safePreview;
        std::wstring source;
        std::wstring relativeTime;
        std::wstring typeLabel;
        std::wstring profile;
        std::wstring representationSummary;
        std::wstring automationName;
        bool pinned{ false };
        bool unavailable{ false };
    };

    struct ManagerSnapshot
    {
        ConnectionState connection{ ConnectionState::Disconnected };
        std::wstring statusTitle;
        std::wstring statusDetail;
        std::wstring activeProfile;
        std::wstring storageSummary;
        std::vector<ClipPreviewData> clips;
        bool synthetic{ false };
    };
}
