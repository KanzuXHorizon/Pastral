#include "pch.h"
#include "HistoryPage.xaml.h"
#include "../Services/ManagerStrings.h"

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
        if (!m_isWideLayout && !m_suppressDetailTransition && HistoryResultsList().SelectedItem())
        {
            m_showingDetails = true;
            UpdateResponsiveLayout();
            HistoryBackButton().Focus(winrt::Microsoft::UI::Xaml::FocusState::Programmatic);
        }
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

    void HistoryPage::BackToResults_Click(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        m_showingDetails = false;
        UpdateResponsiveLayout(true);
    }

    void HistoryPage::Page_SizeChanged(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::SizeChangedEventArgs const& args)
    {
        auto const wasWide = m_isWideLayout;
        m_isWideLayout = args.NewSize().Width >= 920.0;
        if (wasWide && !m_isWideLayout)
        {
            m_showingDetails = false;
            m_suppressDetailTransition = true;
            HistoryResultsList().SelectedIndex(-1);
            m_suppressDetailTransition = false;
            ClearSelectionDetails();
        }
        else if (!wasWide && m_isWideLayout && m_results.Size() > 0 &&
                 HistoryResultsList().SelectedIndex() < 0)
        {
            m_suppressDetailTransition = true;
            HistoryResultsList().SelectedIndex(0);
            m_suppressDetailTransition = false;
            UpdateSelectionDetails();
        }
        UpdateResponsiveLayout();
    }

    void HistoryPage::Page_Loaded(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        m_unloaded = false;
        m_isWideLayout = ActualWidth() >= 920.0;
        UpdateResponsiveLayout();
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
        BeginReadActivity(
            ::Pastral::Manager::Presentation::ManagerStrings::Current().Get(
                L"HistoryActivityRefreshing",
                L"Refreshing local history"));

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
        BeginReadActivity(
            ::Pastral::Manager::Presentation::ManagerStrings::Current().HistoryActivity(
                !query.empty()));

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
        m_showingDetails = false;
        UpdateResponsiveLayout();
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

        auto const& strings = ::Pastral::Manager::Presentation::ManagerStrings::Current();
        HistoryConnectionStatus().Title(strings.StatusTitle(snapshot.statusCode));
        HistoryConnectionStatus().Message(strings.StatusDetail(snapshot.statusCode));
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

        auto const& strings = ::Pastral::Manager::Presentation::ManagerStrings::Current();
        auto const count = m_results.Size();
        auto const hasResults = count > 0;
        HistoryResultCount().Text(strings.FormatItemCount(count, m_hasMore));
        HistoryResultsList().Visibility(hasResults ? Visibility::Visible : Visibility::Collapsed);
        HistoryNoResultsPanel().Visibility(hasResults ? Visibility::Collapsed : Visibility::Visible);
        HistoryClearButton().IsEnabled(!query.empty() && m_connection == ConnectionState::Connected);

        if (hasResults)
        {
            m_showingDetails = false;
            m_suppressDetailTransition = true;
            HistoryResultsList().SelectedIndex(m_isWideLayout ? 0 : -1);
            m_suppressDetailTransition = false;
            if (m_isWideLayout)
            {
                UpdateSelectionDetails();
            }
            else
            {
                ClearSelectionDetails();
            }
        }
        else
        {
            m_showingDetails = false;
            auto const hasQuery = !query.empty();
            HistoryNoResultsTitle().Text(strings.HistoryEmptyTitle(m_connection, hasQuery));
            HistoryNoResultsDetail().Text(strings.HistoryEmptyDetail(m_connection, hasQuery));
            ClearSelectionDetails();
        }
        UpdateResponsiveLayout();
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
        auto const& strings = ::Pastral::Manager::Presentation::ManagerStrings::Current();
        HistoryDetailState().Text(
            selected.StateSummary().empty() ? strings.Available() : selected.StateSummary());
        HistoryAvailabilityWarning().IsOpen(selected.Unavailable());
        HistoryAvailabilityWarning().Visibility(
            selected.Unavailable() ? Visibility::Visible : Visibility::Collapsed);
    }

    void HistoryPage::ClearSelectionDetails()
    {
        using winrt::Microsoft::UI::Xaml::Visibility;

        auto const& strings = ::Pastral::Manager::Presentation::ManagerStrings::Current();
        HistoryDetailPreview().Text(strings.SelectHistoryItem());
        auto const unavailable = strings.Unavailable();
        HistoryDetailSource().Text(unavailable);
        HistoryDetailType().Text(unavailable);
        HistoryDetailRepresentation().Text(unavailable);
        HistoryDetailProfile().Text(unavailable);
        HistoryDetailState().Text(unavailable);
        HistoryAvailabilityWarning().IsOpen(false);
        HistoryAvailabilityWarning().Visibility(Visibility::Collapsed);
    }

    void HistoryPage::UpdateResponsiveLayout(bool restoreResultsFocus)
    {
        using winrt::Microsoft::UI::Xaml::Controls::Control;
        using winrt::Microsoft::UI::Xaml::FocusState;
        using winrt::Microsoft::UI::Xaml::Visibility;

        if (m_isWideLayout)
        {
            HistoryResultsRegion().Visibility(Visibility::Visible);
            HistoryDetailsRegion().Visibility(Visibility::Visible);
            HistoryBackButton().Visibility(Visibility::Collapsed);
            return;
        }

        auto const showDetails = m_showingDetails && HistoryResultsList().SelectedItem();
        HistoryResultsRegion().Visibility(showDetails ? Visibility::Collapsed : Visibility::Visible);
        HistoryDetailsRegion().Visibility(showDetails ? Visibility::Visible : Visibility::Collapsed);
        HistoryBackButton().Visibility(showDetails ? Visibility::Visible : Visibility::Collapsed);

        if (!restoreResultsFocus || showDetails)
        {
            return;
        }

        auto const selected = HistoryResultsList().SelectedItem();
        auto const container = selected
            ? HistoryResultsList().ContainerFromItem(selected).try_as<Control>()
            : nullptr;
        if (container)
        {
            container.Focus(FocusState::Programmatic);
        }
        else
        {
            HistorySearchBox().Focus(FocusState::Programmatic);
        }
    }
}
