#include "pch.h"
#include "HomePage.xaml.h"
#include "../Services/ManagerStrings.h"

#if __has_include("HomePage.g.cpp")
#include "HomePage.g.cpp"
#endif

namespace winrt::Pastral::Manager::implementation
{
    HomePage::HomePage()
        : m_provider(::Pastral::Manager::Presentation::CreateManagerDataProvider())
    {
        InitializeComponent();
        HomeRecentClipsList().ItemsSource(m_recentClips);
        LoadSnapshot();
    }

    void HomePage::RetryConnection_Click(
        winrt::Windows::Foundation::IInspectable const&,
        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&)
    {
        LoadSnapshot(true);
    }

    void HomePage::LoadSnapshot(bool refresh)
    {
        auto const generation = ++m_loadGeneration;
        ApplySnapshot(::Pastral::Manager::Presentation::CreateLoadingSnapshot());

        auto const weakThis = get_weak();
        auto const dispatcher = DispatcherQueue();
        auto completion =
            [weakThis, dispatcher, generation](
                ::Pastral::Manager::Presentation::ManagerSnapshot snapshot) mutable {
                dispatcher.TryEnqueue(
                    [weakThis, generation, snapshot = std::move(snapshot)]() mutable {
                        if (auto strongThis = weakThis.get())
                        {
                            if (strongThis->m_loadGeneration == generation)
                            {
                                strongThis->ApplySnapshot(snapshot);
                            }
                        }
                    });
            };
        if (refresh)
        {
            m_provider->RefreshAsync(std::move(completion));
        }
        else
        {
            m_provider->LoadSnapshotAsync(std::move(completion));
        }
    }

    void HomePage::ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot const& snapshot)
    {
        using ::Pastral::Manager::Presentation::ConnectionState;
        using winrt::Microsoft::UI::Xaml::Controls::InfoBarSeverity;
        using winrt::Microsoft::UI::Xaml::Visibility;

        auto const& strings = ::Pastral::Manager::Presentation::ManagerStrings::Current();
        auto const statusTitle = strings.StatusTitle(snapshot.statusCode);
        auto const statusDetail = strings.StatusDetail(snapshot.statusCode);
        HomeConnectionStatus().Title(statusTitle);
        HomeConnectionStatus().Message(statusDetail);
        InfoBarSeverity severity = InfoBarSeverity::Informational;
        switch (snapshot.connection)
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
        HomeConnectionStatus().Severity(severity);
        auto const showConnectionBanner =
            snapshot.connection != ConnectionState::Loading &&
            snapshot.connection != ConnectionState::Connected;
        HomeConnectionStatus().IsOpen(showConnectionBanner);
        HomeConnectionStatus().Visibility(
            showConnectionBanner ? Visibility::Visible : Visibility::Collapsed);

        HomeStatusTitle().Text(statusTitle);
        HomeStatusDetail().Text(statusDetail);
        HomeProfileValue().Text(strings.ActiveProfile(snapshot.synthetic));
        HomeStorageValue().Text(strings.StorageSummary(snapshot));

        auto const isLoading = snapshot.connection == ConnectionState::Loading;
        HomeLoadingIndicator().IsActive(isLoading);
        HomeLoadingIndicator().Visibility(isLoading ? Visibility::Visible : Visibility::Collapsed);
        HomeStatusIcon().Visibility(isLoading ? Visibility::Collapsed : Visibility::Visible);

        HomeCaptureValue().Text(strings.CaptureValue(snapshot.connection, snapshot.synthetic));
        HomeEmptyStateTitle().Text(strings.HomeEmptyTitle(snapshot.connection, snapshot.synthetic));
        HomeEmptyStateDetail().Text(strings.HomeEmptyDetail(snapshot.connection, snapshot.synthetic));

        HomeSyntheticNotice().IsOpen(snapshot.synthetic);
        HomeSyntheticNotice().Visibility(snapshot.synthetic ? Visibility::Visible : Visibility::Collapsed);
        auto const canRetry =
            snapshot.connection == ConnectionState::Disconnected ||
            snapshot.connection == ConnectionState::ProtocolMismatch ||
            snapshot.connection == ConnectionState::Error;
        auto const canRefresh = snapshot.connection == ConnectionState::Connected && !snapshot.synthetic;
        RetryConnectionButton().Content(winrt::box_value(strings.RetryAction(canRetry)));
        RetryConnectionButton().IsEnabled(canRetry || canRefresh);
        RetryConnectionButton().Visibility(
            (canRetry || canRefresh) ? Visibility::Visible : Visibility::Collapsed);

        m_recentClips.Clear();
        for (auto const& clip : snapshot.clips)
        {
            m_recentClips.Append(winrt::make<ClipPreviewViewModel>(clip));
        }

        auto const hasClips = m_recentClips.Size() > 0;
        HomeRecentClipsList().Visibility(hasClips ? Visibility::Visible : Visibility::Collapsed);
        HomeEmptyStatePanel().Visibility(hasClips ? Visibility::Collapsed : Visibility::Visible);
        HomeRecentCount().Text(strings.FormatItemCount(m_recentClips.Size(), false));
    }
}
