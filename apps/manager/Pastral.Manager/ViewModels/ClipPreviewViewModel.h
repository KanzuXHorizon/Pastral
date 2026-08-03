#pragma once

#include "ClipPreviewViewModel.g.h"
#include "ManagerState.h"

namespace winrt::Pastral::Manager::implementation
{
    struct ClipPreviewViewModel : ClipPreviewViewModelT<ClipPreviewViewModel>
    {
        explicit ClipPreviewViewModel(::Pastral::Manager::Presentation::ClipPreviewData data);

        winrt::hstring Id() const;
        winrt::hstring SafePreview() const;
        winrt::hstring Source() const;
        winrt::hstring RelativeTime() const;
        winrt::hstring TypeLabel() const;
        winrt::hstring Profile() const;
        winrt::hstring RepresentationSummary() const;
        winrt::hstring AutomationName() const;
        winrt::hstring StateSummary() const;
        bool Pinned() const;
        bool Unavailable() const;

    private:
        winrt::hstring m_id;
        winrt::hstring m_safePreview;
        winrt::hstring m_source;
        winrt::hstring m_relativeTime;
        winrt::hstring m_typeLabel;
        winrt::hstring m_profile;
        winrt::hstring m_representationSummary;
        winrt::hstring m_automationName;
        winrt::hstring m_stateSummary;
        bool m_pinned{ false };
        bool m_unavailable{ false };
    };
}
