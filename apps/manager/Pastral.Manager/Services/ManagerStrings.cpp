#include "pch.h"
#include "ManagerStrings.h"

#include <chrono>
#include <initializer_list>
#include <string>

namespace Pastral::Manager::Presentation
{
    namespace
    {
        [[nodiscard]] std::wstring FormatPattern(
            winrt::hstring const& pattern,
            std::initializer_list<std::wstring_view> values)
        {
            std::wstring result{ pattern.c_str() };
            std::size_t index = 0;
            for (auto const value : values)
            {
                auto const token = L"{" + std::to_wstring(index++) + L"}";
                std::size_t position = 0;
                while ((position = result.find(token, position)) != std::wstring::npos)
                {
                    result.replace(position, token.size(), value);
                    position += value.size();
                }
            }
            return result;
        }

        [[nodiscard]] std::wstring Number(std::uint64_t value)
        {
            return std::to_wstring(value);
        }
    }

    ManagerStrings::ManagerStrings() noexcept
    {
        try
        {
            m_loader = winrt::Microsoft::Windows::ApplicationModel::Resources::ResourceLoader();
        }
        catch (...)
        {
            m_loader = nullptr;
        }
    }

    ManagerStrings const& ManagerStrings::Current()
    {
        static ManagerStrings const instance;
        return instance;
    }

    winrt::hstring ManagerStrings::Get(
        std::wstring_view key,
        std::wstring_view fallback) const noexcept
    {
        try
        {
            if (m_loader)
            {
                auto const value = m_loader.GetString(winrt::hstring(key));
                if (!value.empty())
                {
                    return value;
                }
            }
        }
        catch (...)
        {
        }
        return winrt::hstring(fallback);
    }

    winrt::hstring ManagerStrings::FormatItemCount(
        std::uint32_t count,
        bool firstPage) const
    {
        auto const key = count == 1 ? L"CommonItemCountOne" : L"CommonItemCountMany";
        auto const fallback = count == 1 ? L"{0} item" : L"{0} items";
        auto result = FormatPattern(Get(key, fallback), { Number(count) });
        if (firstPage)
        {
            result.append(Get(L"CommonFirstPageSuffix", L" · First page").c_str());
        }
        return winrt::hstring(result);
    }

    winrt::hstring ManagerStrings::StatusTitle(ManagerStatusCode status) const
    {
        switch (status)
        {
        case ManagerStatusCode::Loading:
            return Get(L"StatusLoadingTitle", L"Connecting to local agent");
        case ManagerStatusCode::Connected:
        case ManagerStatusCode::ConnectedFirstPage:
        case ManagerStatusCode::ConnectedCurrentPage:
            return Get(L"StatusConnectedTitle", L"Pastral agent is connected");
        case ManagerStatusCode::Disconnected:
            return Get(L"StatusDisconnectedTitle", L"Pastral agent is not connected");
        case ManagerStatusCode::Timeout:
            return Get(L"StatusTimeoutTitle", L"Pastral agent did not respond");
        case ManagerStatusCode::ProtocolMismatch:
            return Get(L"StatusProtocolMismatchTitle", L"Pastral versions are incompatible");
        case ManagerStatusCode::AuthenticationFailed:
            return Get(L"StatusAuthenticationFailedTitle", L"Pastral agent authentication failed");
        case ManagerStatusCode::Unhealthy:
            return Get(L"StatusUnhealthyTitle", L"Pastral agent needs attention");
        case ManagerStatusCode::InvalidConfiguration:
            return Get(L"StatusInvalidConfigurationTitle", L"Pastral connection configuration is invalid");
        case ManagerStatusCode::AbiMismatch:
            return Get(L"StatusAbiMismatchTitle", L"Pastral bridge versions are incompatible");
        case ManagerStatusCode::HistoryBridgeUnavailable:
            return Get(L"StatusHistoryBridgeUnavailableTitle", L"Pastral history bridge is unavailable");
        case ManagerStatusCode::HistoryChanged:
            return Get(L"StatusHistoryChangedTitle", L"Pastral history changed during refresh");
        case ManagerStatusCode::Synthetic:
            return Get(L"StatusSyntheticTitle", L"Synthetic preview data");
        case ManagerStatusCode::InternalError:
        default:
            return Get(L"StatusInternalErrorTitle", L"Pastral agent connection failed");
        }
    }

    winrt::hstring ManagerStrings::StatusDetail(ManagerStatusCode status) const
    {
        switch (status)
        {
        case ManagerStatusCode::Loading:
            return Get(L"StatusLoadingDetail", L"Checking the secure local connection.");
        case ManagerStatusCode::Connected:
            return Get(L"StatusConnectedDetail", L"The secure local connection is ready and storage checks passed.");
        case ManagerStatusCode::ConnectedFirstPage:
            return Get(L"StatusConnectedFirstPageDetail", L"The secure local connection returned the first bounded page of history.");
        case ManagerStatusCode::ConnectedCurrentPage:
            return Get(L"StatusConnectedCurrentPageDetail", L"The secure local connection returned the current bounded history page.");
        case ManagerStatusCode::Disconnected:
            return Get(L"StatusDisconnectedDetail", L"Start the local agent, then retry the authenticated connection.");
        case ManagerStatusCode::Timeout:
            return Get(L"StatusTimeoutDetail", L"The local agent took too long to respond. Check it, then retry.");
        case ManagerStatusCode::ProtocolMismatch:
            return Get(L"StatusProtocolMismatchDetail", L"The manager and local agent use incompatible connection versions.");
        case ManagerStatusCode::AuthenticationFailed:
            return Get(L"StatusAuthenticationFailedDetail", L"The local agent identity could not be authenticated. Restart or repair the installation before retrying.");
        case ManagerStatusCode::Unhealthy:
            return Get(L"StatusUnhealthyDetail", L"The agent reported a privacy-policy or storage-integrity failure. History remains unavailable.");
        case ManagerStatusCode::InvalidConfiguration:
            return Get(L"StatusInvalidConfigurationDetail", L"The local data location is not valid for the secure connection.");
        case ManagerStatusCode::AbiMismatch:
            return Get(L"StatusAbiMismatchDetail", L"The manager and local IPC bridge use different native interface versions.");
        case ManagerStatusCode::HistoryBridgeUnavailable:
            return Get(L"StatusHistoryBridgeUnavailableDetail", L"Repair the installation so the manager can load the bounded History interface.");
        case ManagerStatusCode::HistoryChanged:
            return Get(L"StatusHistoryChangedDetail", L"The bounded history page changed too quickly to copy safely. Retry the request.");
        case ManagerStatusCode::Synthetic:
            return Get(L"StatusSyntheticDetail", L"These bounded examples exercise manager layout and accessibility. They are not clipboard history.");
        case ManagerStatusCode::InternalError:
        default:
            return Get(L"StatusInternalErrorDetail", L"The manager could not complete the secure local connection check.");
        }
    }

    winrt::hstring ManagerStrings::StorageSummary(ManagerSnapshot const& snapshot) const
    {
        switch (snapshot.statusCode)
        {
        case ManagerStatusCode::Loading:
            return Get(L"StoragePreparing", L"Preparing local status");
        case ManagerStatusCode::Connected:
        case ManagerStatusCode::ConnectedFirstPage:
        case ManagerStatusCode::ConnectedCurrentPage:
        {
            auto const pattern = Get(
                L"StorageConnected",
                L"Schema {0} · Integrity verified · {1}");
            auto const schema = Number(snapshot.storageSchemaVersion);
            auto const count = FormatItemCount(
                static_cast<std::uint32_t>(snapshot.clips.size()),
                false);
            return winrt::hstring(FormatPattern(pattern, { schema, std::wstring_view(count.c_str()) }));
        }
        case ManagerStatusCode::Disconnected:
            return Get(L"StorageDisconnected", L"Unavailable until the local agent is connected");
        case ManagerStatusCode::Timeout:
            return Get(L"StorageTimeout", L"Unavailable because the local agent timed out");
        case ManagerStatusCode::ProtocolMismatch:
        case ManagerStatusCode::AbiMismatch:
            return Get(L"StorageVersionMismatch", L"Unavailable until manager and agent versions match");
        case ManagerStatusCode::Synthetic:
            return Get(L"StorageSynthetic", L"Synthetic examples only · No database or blob access");
        default:
            return Get(L"StorageUnhealthy", L"Unavailable until the local agent is healthy");
        }
    }

    winrt::hstring ManagerStrings::ActiveProfile(bool synthetic) const
    {
        return synthetic
            ? Get(L"ProfileDevelopment", L"Development")
            : Get(L"ProfileOrdinary", L"Ordinary");
    }

    winrt::hstring ManagerStrings::CaptureValue(
        ConnectionState state,
        bool synthetic) const
    {
        switch (state)
        {
        case ConnectionState::Loading:
            return Get(L"CaptureConnecting", L"Connecting securely");
        case ConnectionState::Connected:
            return synthetic
                ? Get(L"CapturePreviewMode", L"Preview mode")
                : Get(L"CaptureConnected", L"Connected");
        case ConnectionState::CapturePaused:
            return Get(L"CapturePaused", L"Paused");
        case ConnectionState::ProtocolMismatch:
            return Get(L"CaptureVersionMismatch", L"Version mismatch");
        case ConnectionState::Error:
            return Get(L"CaptureNeedsAttention", L"Needs attention");
        case ConnectionState::Disconnected:
        default:
            return Get(L"CommonUnavailable", L"Unavailable");
        }
    }

    winrt::hstring ManagerStrings::HomeEmptyTitle(
        ConnectionState state,
        bool synthetic) const
    {
        switch (state)
        {
        case ConnectionState::Loading:
            return Get(L"HomeEmptyLoadingTitle", L"Connecting to local agent");
        case ConnectionState::Connected:
            return synthetic
                ? Get(L"HomeEmptySyntheticTitle", L"No synthetic previews are available")
                : Get(L"HomeEmptyConnectedTitle", L"No clipboard history yet");
        case ConnectionState::CapturePaused:
            return Get(L"HomeEmptyPausedTitle", L"Capture is paused");
        default:
            return Get(L"HomeEmptyUnavailableTitle", L"Recent clips are unavailable");
        }
    }

    winrt::hstring ManagerStrings::HomeEmptyDetail(
        ConnectionState state,
        bool synthetic) const
    {
        switch (state)
        {
        case ConnectionState::Loading:
            return Get(L"HomeEmptyLoadingDetail", L"Recent clips will appear after the secure local connection is ready.");
        case ConnectionState::Connected:
            return synthetic
                ? Get(L"HomeEmptySyntheticDetail", L"The Debug presentation provider returned no bounded preview records.")
                : Get(L"HomeEmptyConnectedDetail", L"New safe clipboard previews will appear after the local agent captures them.");
        case ConnectionState::Disconnected:
            return Get(L"HomeEmptyDisconnectedDetail", L"Start the local agent, then retry the authenticated connection.");
        case ConnectionState::CapturePaused:
            return Get(L"HomeEmptyPausedDetail", L"Resume capture from the agent before expecting new clipboard activity.");
        case ConnectionState::ProtocolMismatch:
            return Get(L"HomeEmptyProtocolDetail", L"Update the manager and agent to compatible versions, then retry.");
        case ConnectionState::Error:
        default:
            return Get(L"HomeEmptyErrorDetail", L"Resolve the manager status above before requesting clipboard history.");
        }
    }

    winrt::hstring ManagerStrings::RetryAction(bool retry) const
    {
        return retry
            ? Get(L"CommonRetry", L"Retry")
            : Get(L"CommonRefresh", L"Refresh");
    }

    winrt::hstring ManagerStrings::HistoryActivity(bool searching) const
    {
        return searching
            ? Get(L"HistoryActivitySearching", L"Searching local history")
            : Get(L"HistoryActivityLoading", L"Loading local history");
    }

    winrt::hstring ManagerStrings::HistoryEmptyTitle(
        ConnectionState state,
        bool hasQuery) const
    {
        switch (state)
        {
        case ConnectionState::Loading:
            return Get(L"HistoryEmptyLoadingTitle", L"Loading local history");
        case ConnectionState::Disconnected:
            return Get(L"HistoryEmptyDisconnectedTitle", L"History is not connected");
        case ConnectionState::ProtocolMismatch:
        case ConnectionState::Error:
            return Get(L"HistoryEmptyUnavailableTitle", L"History is unavailable");
        default:
            return hasQuery
                ? Get(L"HistoryEmptyNoMatchTitle", L"No matching clips")
                : Get(L"HistoryEmptyNoHistoryTitle", L"No clipboard history yet");
        }
    }

    winrt::hstring ManagerStrings::HistoryEmptyDetail(
        ConnectionState state,
        bool hasQuery) const
    {
        switch (state)
        {
        case ConnectionState::Loading:
            return Get(L"HistoryEmptyLoadingDetail", L"The manager is checking the authenticated local connection and first bounded page.");
        case ConnectionState::Disconnected:
            return Get(L"HistoryEmptyDisconnectedDetail", L"Start the local agent, then retry. The manager never opens storage directly.");
        case ConnectionState::ProtocolMismatch:
        case ConnectionState::Error:
            return Get(L"HistoryEmptyUnavailableDetail", L"Resolve the local connection issue before requesting history.");
        default:
            return hasQuery
                ? Get(L"HistoryEmptyNoMatchDetail", L"Check the literal search text or clear it to return to recent safe previews.")
                : Get(L"HistoryEmptyNoHistoryDetail", L"New safe clipboard previews will appear here after the local agent captures them.");
        }
    }

    winrt::hstring ManagerStrings::RelativeTime(std::int64_t observedAtUnixMicros) const
    {
        using namespace std::chrono;
        auto const now = duration_cast<microseconds>(system_clock::now().time_since_epoch()).count();
        auto const elapsed = now > observedAtUnixMicros ? now - observedAtUnixMicros : 0;
        auto const seconds = static_cast<std::uint64_t>(elapsed / 1'000'000);
        if (seconds < 60)
        {
            return Get(L"RelativeJustNow", L"Just now");
        }
        auto const minutes = seconds / 60;
        if (minutes < 60)
        {
            return winrt::hstring(FormatPattern(
                Get(L"RelativeMinutes", L"{0} min ago"),
                { Number(minutes) }));
        }
        auto const hours = minutes / 60;
        if (hours < 24)
        {
            auto const key = hours == 1 ? L"RelativeHourOne" : L"RelativeHoursMany";
            auto const fallback = hours == 1 ? L"{0} hour ago" : L"{0} hours ago";
            return winrt::hstring(FormatPattern(Get(key, fallback), { Number(hours) }));
        }
        auto const days = hours / 24;
        if (days < 30)
        {
            auto const key = days == 1 ? L"RelativeDayOne" : L"RelativeDaysMany";
            auto const fallback = days == 1 ? L"{0} day ago" : L"{0} days ago";
            return winrt::hstring(FormatPattern(Get(key, fallback), { Number(days) }));
        }
        return Get(L"RelativeMonthPlus", L"More than a month ago");
    }

    winrt::hstring ManagerStrings::ClipType(std::wstring_view raw) const
    {
        if (raw == L"Text") return Get(L"ClipTypeText", L"Text");
        if (raw == L"Code") return Get(L"ClipTypeCode", L"Code");
        if (raw == L"Link") return Get(L"ClipTypeLink", L"Link");
        if (raw == L"Image") return Get(L"ClipTypeImage", L"Image");
        if (raw == L"File reference") return Get(L"ClipTypeFileReference", L"File reference");
        if (raw == L"Unavailable") return Get(L"CommonUnavailable", L"Unavailable");
        return winrt::hstring(raw);
    }

    winrt::hstring ManagerStrings::Profile(std::wstring_view raw) const
    {
        if (raw == L"Development") return Get(L"ProfileDevelopment", L"Development");
        if (raw == L"Ordinary") return Get(L"ProfileOrdinary", L"Ordinary");
        return winrt::hstring(raw);
    }

    winrt::hstring ManagerStrings::Representation(std::wstring_view raw) const
    {
        if (raw == L"Unicode text · Plain text") return Get(L"RepresentationTextPlain", L"Unicode text · Plain text");
        if (raw == L"Unicode text · Web link") return Get(L"RepresentationWebLink", L"Unicode text · Web link");
        if (raw == L"PNG · Bitmap") return Get(L"RepresentationImage", L"PNG · Bitmap");
        if (raw == L"Reference only") return Get(L"RepresentationReferenceOnly", L"Reference only");
        if (raw == L"Preview metadata only · Content unavailable") return Get(L"RepresentationUnavailable", L"Preview metadata only · Content unavailable");
        if (raw == L"Text preview · Truncated") return Get(L"RepresentationTextTruncated", L"Text preview · Truncated");
        if (raw == L"Text preview") return Get(L"RepresentationTextPreview", L"Text preview");
        return winrt::hstring(raw);
    }

    winrt::hstring ManagerStrings::StateSummary(
        bool pinned,
        bool unavailable,
        bool truncated) const
    {
        if (pinned && unavailable) return Get(L"StatePinnedUnavailable", L"Pinned · Unavailable");
        if (pinned && truncated) return Get(L"StatePinnedTruncated", L"Pinned · Preview truncated");
        if (pinned) return Get(L"StatePinned", L"Pinned");
        if (unavailable) return Get(L"CommonUnavailable", L"Unavailable");
        if (truncated) return Get(L"StateTruncated", L"Preview truncated");
        return Available();
    }

    winrt::hstring ManagerStrings::ClipAutomationName(
        std::wstring_view safePreview,
        std::wstring_view source,
        std::wstring_view relativeTime,
        std::wstring_view typeLabel,
        std::wstring_view profile,
        std::wstring_view stateSummary) const
    {
        auto const pattern = Get(
            L"ClipAutomationName",
            L"{0}. Source: {1}. Type: {2}. Profile: {3}. Time: {4}. State: {5}.");
        return winrt::hstring(FormatPattern(
            pattern,
            { safePreview, source, typeLabel, profile, relativeTime, stateSummary }));
    }

    winrt::hstring ManagerStrings::Available() const
    {
        return Get(L"CommonAvailable", L"Available");
    }

    winrt::hstring ManagerStrings::Unavailable() const
    {
        return Get(L"CommonUnavailable", L"Unavailable");
    }

    winrt::hstring ManagerStrings::SelectHistoryItem() const
    {
        return Get(L"HistorySelectItem", L"Select a history item");
    }
}
