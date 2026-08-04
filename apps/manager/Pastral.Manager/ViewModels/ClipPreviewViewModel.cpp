#include "pch.h"
#include "ClipPreviewViewModel.h"

#if __has_include("ClipPreviewViewModel.g.cpp")
#include "ClipPreviewViewModel.g.cpp"
#endif

namespace winrt::Pastral::Manager::implementation
{
    ClipPreviewViewModel::ClipPreviewViewModel(::Pastral::Manager::Presentation::ClipPreviewData data)
        : m_id(std::move(data.id)),
          m_safePreview(std::move(data.safePreview)),
          m_source(std::move(data.source)),
          m_relativeTime(std::move(data.relativeTime)),
          m_typeLabel(std::move(data.typeLabel)),
          m_profile(std::move(data.profile)),
          m_representationSummary(std::move(data.representationSummary)),
          m_automationName(std::move(data.automationName)),
          m_pinned(data.pinned),
          m_unavailable(data.unavailable)
    {
        if (m_pinned && m_unavailable)
        {
            m_stateSummary = L"Pinned · Unavailable";
        }
        else if (m_pinned && data.previewTruncated)
        {
            m_stateSummary = L"Pinned · Preview truncated";
        }
        else if (m_pinned)
        {
            m_stateSummary = L"Pinned";
        }
        else if (m_unavailable)
        {
            m_stateSummary = L"Unavailable";
        }
        else if (data.previewTruncated)
        {
            m_stateSummary = L"Preview truncated";
        }
    }

    winrt::hstring ClipPreviewViewModel::Id() const
    {
        return m_id;
    }

    winrt::hstring ClipPreviewViewModel::SafePreview() const
    {
        return m_safePreview;
    }

    winrt::hstring ClipPreviewViewModel::Source() const
    {
        return m_source;
    }

    winrt::hstring ClipPreviewViewModel::RelativeTime() const
    {
        return m_relativeTime;
    }

    winrt::hstring ClipPreviewViewModel::TypeLabel() const
    {
        return m_typeLabel;
    }

    winrt::hstring ClipPreviewViewModel::Profile() const
    {
        return m_profile;
    }

    winrt::hstring ClipPreviewViewModel::RepresentationSummary() const
    {
        return m_representationSummary;
    }

    winrt::hstring ClipPreviewViewModel::AutomationName() const
    {
        return m_automationName;
    }

    winrt::hstring ClipPreviewViewModel::StateSummary() const
    {
        return m_stateSummary;
    }

    bool ClipPreviewViewModel::Pinned() const
    {
        return m_pinned;
    }

    bool ClipPreviewViewModel::Unavailable() const
    {
        return m_unavailable;
    }
}
