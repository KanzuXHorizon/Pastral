#include "pch.h"
#include "HistoryPage.xaml.h"

#if __has_include("HistoryPage.g.cpp")
#include "HistoryPage.g.cpp"
#endif

#include <chrono>

namespace winrt::Pastral::Manager::implementation
{
    HistoryPage::HistoryPage()
        : m_provider(::Pastral::Manager::Presentation::CreateManagerDataProvider())
    {
        InitializeComponent();
        HistoryResultsList().ItemsSource(m_results);
        m_searchTimer = DispatcherQueue().CreateTimer();
        m_searchTimer.Interval(std::chrono::milliseconds(250));
        m_searchTimer.IsRepeating(false);
        auto const weakThis = get_weak();
        m_searchTimer.Tick([weakThis](auto const&, auto const&) {
            if (auto strongThis = weakThis.get())
            {
                if (!strongThis->m_unloaded)
                {
                    strongThis->SearchSnapshot(
                        std::wstring(strongThis->HistorySearchBox().Text().c_str()));
                }
            }
        });
        LoadSnapshot();
    }

    void HistoryPage::SearchBox_TextChanged(
        winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBox const&,
        winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBoxTextChangedEventArgs const&)
    {
        if (m_unloaded || !m_searchTimer)
        {
            return;
        }
        ++m_loadGeneration;
        m_searchTimer.Stop();
        m_searchTimer.Start();
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
        if (m_unloaded)
        {
            return;
        }
        HistorySearchBox().Text(L"");
        m_searchTimer.Stop();
        SearchSnapshot({});
    }

    void HistoryPage::Retry_Click(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        if (m_unloaded)
        {
            return;
        }
        HistorySearchBox().Text(L"");
        m_searchTimer.Stop();
        RefreshSnapshot();
    }

    void HistoryPage::Page_Loaded(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        m_unloaded = false;
        if (m_hasLoadedOnce)
        {
            RefreshSnapshot();
        }
        else
        {
            m_hasLoadedOnce = true;
        }
    }

    void HistoryPage::Page_Unloaded(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        m_unloaded = true;
        ++m_loadGeneration;
        if (m_searchTimer)
        {
            m_searchTimer.Stop();
        }
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
                            if (!strongThis->m_unloaded &&
                                strongThis->m_loadGeneration == generation)
                            {
                                strongThis->ApplySnapshot(std::move(snapshot));
                            }
                        }
                    });
            });
    }

    void HistoryPage::RefreshSnapshot()
    {
        auto const generation = ++m_loadGeneration;
        BeginReadActivity(L"Refreshing local history");

        auto const weakThis = get_weak();
        auto const dispatcher = DispatcherQueue();
        m_provider->RefreshAsync(
            [weakThis, dispatcher, generation](
                ::Pastral::Manager::Presentation::ManagerSnapshot snapshot) mutable {
                dispatcher.TryEnqueue(
                    [weakThis, generation, snapshot = std::move(snapshot)]() mutable {
                        if (auto strongThis = weakThis.get())
                        {
                            if (!strongThis->m_unloaded &&
                                strongThis->m_loadGeneration == generation)
                            {
                                strongThis->ApplySnapshot(std::move(snapshot));
                            }
                        }
                    });
            });
    }

    void HistoryPage::SearchSnapshot(std::wstring query)
    {
        auto const generation = ++m_loadGeneration;
        BeginReadActivity(query.empty() ? L"Loading local history" : L"Searching local history");

        auto const weakThis = get_weak();
        auto const dispatcher = DispatcherQueue();
        m_provider->SearchAsync(
            std::move(query),
            [weakThis, dispatcher, generation](
                ::Pastral::Manager::Presentation::ManagerSnapshot snapshot) mutable {
                dispatcher.TryEnqueue(
                    [weakThis, generation, snapshot = std::move(snapshot)]() mutable {
                        if (auto strongThis = weakThis.get())
                        {
                            if (!strongThis->m_unloaded &&
                                strongThis->m_loadGeneration == generation)
                            {
                                strongThis->ApplySnapshot(std::move(snapshot));
                            }
                        }
                    });
            });
    }

    void HistoryPage::BeginReadActivity(winrt::hstring const& announcement)
    {
        using winrt::Microsoft::UI::Xaml::Visibility;
        HistoryLoadingIndicator().IsActive(true);
        HistoryLoadingIndicator().Visibility(Visibility::Visible);
        HistoryResultCount().Text(announcement);
        HistorySearchBox().IsEnabled(m_connection ==
            ::Pastral::Manager::Presentation::ConnectionState::Connected);
    }

    void HistoryPage::ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot snapshot)
    {
        using ::Pastral::Manager::Presentation::ConnectionState;
        using winrt::Microsoft::UI::Xaml::Controls::InfoBarSeverity;
        using winrt::Microsoft::UI::Xaml::Visibility;

        m_connection = snapshot.connection;
        m_synthetic = snapshot.synthetic;
        m_hasMore = snapshot.hasMore;
        m_allClips = std::move(snapshot.clips);

        HistoryConnectionStatus().Title(winrt::hstring(snapshot.statusTitle));
        HistoryConnectionStatus().Message(winrt::hstring(snapshot.statusDetail));
        InfoBarSeverity severity = InfoBarSeverity::Informational;
        switch (m_connection)
        {
        case ConnectionState::Connected:
            severity = InfoBarSeverity::Success;
            break;
        case ConnectionState::Disconnected:
        case ConnectionState::CapturePaused:
            severity = InfoBarSeverity::Warning;
            break;
        case ConnectionState::ProtocolMismatch:
        case ConnectionState::Error:
            severity = InfoBarSeverity::Error;
            break;
        case ConnectionState::Loading:
            break;
        }
        HistoryConnectionStatus().Severity(severity);
        auto const showConnectionBanner =
            m_connection != ConnectionState::Loading &&
            m_connection != ConnectionState::Connected;
        HistoryConnectionStatus().IsOpen(showConnectionBanner);
        HistoryConnectionStatus().Visibility(
            showConnectionBanner ? Visibility::Visible : Visibility::Collapsed);

        HistorySyntheticNotice().IsOpen(m_synthetic);
        HistorySyntheticNotice().Visibility(m_synthetic ? Visibility::Visible : Visibility::Collapsed);

        auto const connected = m_connection == ConnectionState::Connected;
        HistorySearchBox().IsEnabled(connected);
        auto const canRetry =
            m_connection == ConnectionState::Disconnected ||
            m_connection == ConnectionState::ProtocolMismatch ||
            m_connection == ConnectionState::Error;
        HistoryRetryButton().IsEnabled(canRetry);
        HistoryRetryButton().Visibility(canRetry ? Visibility::Visible : Visibility::Collapsed);

        auto const isLoading = m_connection == ConnectionState::Loading;
        HistoryLoadingIndicator().IsActive(isLoading);
        HistoryLoadingIndicator().Visibility(isLoading ? Visibility::Visible : Visibility::Collapsed);
        HistoryEmptyIcon().Visibility(isLoading ? Visibility::Collapsed : Visibility::Visible);

        RefreshResults();
    }

    void HistoryPage::RefreshResults()
    {
        using ::Pastral::Manager::Presentation::ConnectionState;
        using winrt::Microsoft::UI::Xaml::Visibility;

        auto const query = std::wstring(HistorySearchBox().Text().c_str());

        m_results.Clear();
        if (m_connection == ConnectionState::Connected)
        {
            for (auto const& clip : m_allClips)
            {
                m_results.Append(winrt::make<ClipPreviewViewModel>(clip));
            }
        }

        auto const count = m_results.Size();
        auto const hasResults = count > 0;
        auto countText = std::to_wstring(count) + (count == 1 ? L" item" : L" items");
        if (m_hasMore)
        {
            countText += L" · First page";
        }
        HistoryResultCount().Text(winrt::hstring(countText));
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
                HistoryNoResultsTitle().Text(L"Loading local history");
                HistoryNoResultsDetail().Text(
                    L"The manager is checking the authenticated local connection and first bounded page.");
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
                    L"Resolve the local connection issue before requesting history.");
            }
            else if (!query.empty())
            {
                HistoryNoResultsTitle().Text(L"No matching clips");
                HistoryNoResultsDetail().Text(
                    L"Check the literal search text or clear it to return to recent safe previews.");
            }
            else
            {
                HistoryNoResultsTitle().Text(L"No clipboard history yet");
                HistoryNoResultsDetail().Text(
                    L"New safe clipboard previews will appear here after the local agent captures them.");
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
