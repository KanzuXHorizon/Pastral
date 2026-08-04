#include "pch.h"
#include "HistoryPage.xaml.h"

#if __has_include("HistoryPage.g.cpp")
#include "HistoryPage.g.cpp"
#endif

#include <cwctype>

namespace
{
    std::wstring Lowercase(std::wstring_view value)
    {
        std::wstring lowered;
        lowered.reserve(value.size());
        for (auto const character : value)
        {
            lowered.push_back(static_cast<wchar_t>(std::towlower(character)));
        }
        return lowered;
    }

    bool Contains(std::wstring const& value, std::wstring const& loweredQuery)
    {
        return Lowercase(value).find(loweredQuery) != std::wstring::npos;
    }

    bool Matches(
        Pastral::Manager::Presentation::ClipPreviewData const& clip,
        std::wstring const& loweredQuery)
    {
        if (loweredQuery.empty())
        {
            return true;
        }

        return Contains(clip.safePreview, loweredQuery) ||
            Contains(clip.source, loweredQuery) ||
            Contains(clip.typeLabel, loweredQuery) ||
            Contains(clip.profile, loweredQuery) ||
            Contains(clip.representationSummary, loweredQuery);
    }
}

namespace winrt::Pastral::Manager::implementation
{
    HistoryPage::HistoryPage()
        : m_provider(::Pastral::Manager::Presentation::CreateManagerDataProvider())
    {
        InitializeComponent();
        HistoryResultsList().ItemsSource(m_results);
        LoadSnapshot();
    }

    void HistoryPage::SearchBox_TextChanged(
        winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBox const&,
        winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBoxTextChangedEventArgs const&)
    {
        RefreshResults();
    }

    void HistoryPage::ResultsList_SelectionChanged(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const&)
    {
        UpdateSelectionDetails();
    }

    void HistoryPage::ClearFilters_Click(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        if (HistorySearchBox().Text().empty())
        {
            RefreshResults();
            return;
        }

        HistorySearchBox().Text(L"");
    }

    void HistoryPage::Retry_Click(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        LoadSnapshot();
    }

    void HistoryPage::LoadSnapshot()
    {
        auto const generation = ++m_loadGeneration;
        ApplySnapshot(::Pastral::Manager::Presentation::CreateLoadingSnapshot());

        auto const weakThis = get_weak();
        auto const dispatcher = DispatcherQueue();
        m_provider->LoadSnapshotAsync(
            [weakThis, dispatcher, generation](
                ::Pastral::Manager::Presentation::ManagerSnapshot snapshot) mutable {
                dispatcher.TryEnqueue(
                    [weakThis, generation, snapshot = std::move(snapshot)]() mutable {
                        if (auto strongThis = weakThis.get())
                        {
                            if (strongThis->m_loadGeneration == generation)
                            {
                                strongThis->ApplySnapshot(std::move(snapshot));
                            }
                        }
                    });
            });
    }

    void HistoryPage::ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot snapshot)
    {
        using ::Pastral::Manager::Presentation::ConnectionState;
        using winrt::Microsoft::UI::Xaml::Controls::InfoBarSeverity;
        using winrt::Microsoft::UI::Xaml::Visibility;

        m_connection = snapshot.connection;
        m_synthetic = snapshot.synthetic;
        m_allClips = std::move(snapshot.clips);

        HistoryConnectionStatus().Title(winrt::hstring(snapshot.statusTitle));
        HistoryConnectionStatus().Message(winrt::hstring(snapshot.statusDetail));
        HistoryConnectionStatus().Severity(
            m_connection == ConnectionState::Error ||
            m_connection == ConnectionState::ProtocolMismatch
                ? InfoBarSeverity::Error
                : InfoBarSeverity::Informational);
        HistoryConnectionStatus().IsOpen(true);

        HistorySyntheticNotice().IsOpen(m_synthetic);
        HistorySyntheticNotice().Visibility(m_synthetic ? Visibility::Visible : Visibility::Collapsed);

        auto const connected = m_connection == ConnectionState::Connected;
        HistorySearchBox().IsEnabled(connected);
        auto const canRetry =
            m_connection == ConnectionState::Disconnected ||
            m_connection == ConnectionState::ProtocolMismatch ||
            m_connection == ConnectionState::Error;
        HistoryRetryButton().Visibility(canRetry ? Visibility::Visible : Visibility::Collapsed);

        RefreshResults();
    }

    void HistoryPage::RefreshResults()
    {
        using ::Pastral::Manager::Presentation::ConnectionState;
        using winrt::Microsoft::UI::Xaml::Visibility;

        auto const query = std::wstring(HistorySearchBox().Text().c_str());
        auto const loweredQuery = Lowercase(query);

        m_results.Clear();
        if (m_connection == ConnectionState::Connected)
        {
            for (auto const& clip : m_allClips)
            {
                if (Matches(clip, loweredQuery))
                {
                    m_results.Append(winrt::make<ClipPreviewViewModel>(clip));
                }
            }
        }

        auto const count = m_results.Size();
        auto const hasResults = count > 0;
        HistoryResultCount().Text(winrt::hstring(std::to_wstring(count) + L" items"));
        HistoryResultsList().Visibility(hasResults ? Visibility::Visible : Visibility::Collapsed);
        HistoryNoResultsPanel().Visibility(hasResults ? Visibility::Collapsed : Visibility::Visible);
        HistoryClearButton().IsEnabled(!query.empty() && m_connection == ConnectionState::Connected);

        if (hasResults)
        {
            HistoryResultsList().SelectedIndex(0);
        }
        else
        {
            if (m_connection == ConnectionState::Loading)
            {
                HistoryNoResultsTitle().Text(L"Connecting to Pastral agent");
                HistoryNoResultsDetail().Text(
                    L"The manager is verifying the authenticated local Health endpoint.");
            }
            else if (m_connection == ConnectionState::Disconnected)
            {
                HistoryNoResultsTitle().Text(L"History is not connected");
                HistoryNoResultsDetail().Text(
                    L"Start the local agent, then retry. The manager never opens storage directly.");
            }
            else if (m_connection == ConnectionState::ProtocolMismatch ||
                     m_connection == ConnectionState::Error)
            {
                HistoryNoResultsTitle().Text(L"History is unavailable");
                HistoryNoResultsDetail().Text(
                    L"Resolve the agent connection issue before requesting history.");
            }
            else if (!query.empty())
            {
                HistoryNoResultsTitle().Text(L"No matching clips");
                HistoryNoResultsDetail().Text(L"Check the search text or clear it to return to all safe previews.");
            }
            else
            {
                HistoryNoResultsTitle().Text(L"History is not available yet");
                HistoryNoResultsDetail().Text(
                    L"The authenticated Health connection is active. Paged history IPC is the next implementation stage.");
            }
            ClearSelectionDetails();
        }
    }

    void HistoryPage::UpdateSelectionDetails()
    {
        using winrt::Microsoft::UI::Xaml::Visibility;

        auto const selected = HistoryResultsList().SelectedItem()
            .try_as<winrt::Pastral::Manager::ClipPreviewViewModel>();
        if (!selected)
        {
            ClearSelectionDetails();
            return;
        }

        HistoryDetailPreview().Text(selected.SafePreview());
        HistoryDetailSource().Text(selected.Source() + L" · " + selected.RelativeTime());
        HistoryDetailType().Text(selected.TypeLabel());
        HistoryDetailRepresentation().Text(selected.RepresentationSummary());
        HistoryDetailProfile().Text(selected.Profile());
        HistoryDetailState().Text(selected.StateSummary().empty() ? L"Available" : selected.StateSummary());
        HistoryAvailabilityWarning().IsOpen(selected.Unavailable());
        HistoryAvailabilityWarning().Visibility(
            selected.Unavailable() ? Visibility::Visible : Visibility::Collapsed);
    }

    void HistoryPage::ClearSelectionDetails()
    {
        using winrt::Microsoft::UI::Xaml::Visibility;

        HistoryDetailPreview().Text(L"Select a history item");
        HistoryDetailSource().Text(L"Unavailable");
        HistoryDetailType().Text(L"Unavailable");
        HistoryDetailRepresentation().Text(L"Unavailable");
        HistoryDetailProfile().Text(L"Unavailable");
        HistoryDetailState().Text(L"Unavailable");
        HistoryAvailabilityWarning().IsOpen(false);
        HistoryAvailabilityWarning().Visibility(Visibility::Collapsed);
    }
}
