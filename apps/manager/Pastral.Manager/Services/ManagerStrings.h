#pragma once

#include "../ViewModels/ManagerState.h"

#include <cstdint>
#include <string_view>

namespace Pastral::Manager::Presentation
{
    class ManagerStrings final
    {
    public:
        [[nodiscard]] static ManagerStrings const& Current();

        [[nodiscard]] winrt::hstring Get(
            std::wstring_view key,
            std::wstring_view fallback) const noexcept;
        [[nodiscard]] winrt::hstring FormatItemCount(
            std::uint32_t count,
            bool firstPage) const;
        [[nodiscard]] winrt::hstring StatusTitle(ManagerStatusCode status) const;
        [[nodiscard]] winrt::hstring StatusDetail(ManagerStatusCode status) const;
        [[nodiscard]] winrt::hstring StorageSummary(ManagerSnapshot const& snapshot) const;
        [[nodiscard]] winrt::hstring ActiveProfile(bool synthetic) const;
        [[nodiscard]] winrt::hstring CaptureValue(
            ConnectionState state,
            bool synthetic) const;
        [[nodiscard]] winrt::hstring HomeEmptyTitle(
            ConnectionState state,
            bool synthetic) const;
        [[nodiscard]] winrt::hstring HomeEmptyDetail(
            ConnectionState state,
            bool synthetic) const;
        [[nodiscard]] winrt::hstring RetryAction(bool retry) const;
        [[nodiscard]] winrt::hstring HistoryActivity(bool searching) const;
        [[nodiscard]] winrt::hstring HistoryEmptyTitle(
            ConnectionState state,
            bool hasQuery) const;
        [[nodiscard]] winrt::hstring HistoryEmptyDetail(
            ConnectionState state,
            bool hasQuery) const;
        [[nodiscard]] winrt::hstring RelativeTime(std::int64_t observedAtUnixMicros) const;
        [[nodiscard]] winrt::hstring ClipType(std::wstring_view raw) const;
        [[nodiscard]] winrt::hstring Profile(std::wstring_view raw) const;
        [[nodiscard]] winrt::hstring Representation(std::wstring_view raw) const;
        [[nodiscard]] winrt::hstring StateSummary(
            bool pinned,
            bool unavailable,
            bool truncated) const;
        [[nodiscard]] winrt::hstring ClipAutomationName(
            std::wstring_view safePreview,
            std::wstring_view source,
            std::wstring_view relativeTime,
            std::wstring_view typeLabel,
            std::wstring_view profile,
            std::wstring_view stateSummary) const;
        [[nodiscard]] winrt::hstring Available() const;
        [[nodiscard]] winrt::hstring Unavailable() const;
        [[nodiscard]] winrt::hstring SelectHistoryItem() const;

    private:
        ManagerStrings() noexcept;

        winrt::Microsoft::Windows::ApplicationModel::Resources::ResourceLoader m_loader{ nullptr };
    };
}
