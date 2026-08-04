#include "pch.h"
#include "HomePage.xaml.h"

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
        LoadSnapshot();
    }

    void HomePage::LoadSnapshot()
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
                                strongThis->ApplySnapshot(snapshot);
                            }
                        }
                    });
            });
    }

    void HomePage::ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot const& snapshot)
    {
        using ::Pastral::Manager::Presentation::ConnectionState;
        using winrt::Microsoft::UI::Xaml::Controls::InfoBarSeverity;
        using winrt::Microsoft::UI::Xaml::Visibility;

        HomeConnectionStatus().Title(winrt::hstring(snapshot.statusTitle));
        HomeConnectionStatus().Message(winrt::hstring(snapshot.statusDetail));
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
        auto const showConnectionBanner = snapshot.connection != ConnectionState::Connected;
        HomeConnectionStatus().IsOpen(showConnectionBanner);
        HomeConnectionStatus().Visibility(
            showConnectionBanner ? Visibility::Visible : Visibility::Collapsed);

        HomeStatusTitle().Text(winrt::hstring(snapshot.statusTitle));
        HomeStatusDetail().Text(winrt::hstring(snapshot.statusDetail));
        HomeProfileValue().Text(winrt::hstring(snapshot.activeProfile));
        HomeStorageValue().Text(winrt::hstring(snapshot.storageSummary));

        auto const isLoading = snapshot.connection == ConnectionState::Loading;
        HomeLoadingIndicator().IsActive(isLoading);
        HomeLoadingIndicator().Visibility(isLoading ? Visibility::Visible : Visibility::Collapsed);
        HomeStatusIcon().Visibility(isLoading ? Visibility::Collapsed : Visibility::Visible);

        switch (snapshot.connection)
        {
        case ConnectionState::Loading:
            HomeCaptureValue().Text(L"Checking agent");
            HomeEmptyStateTitle().Text(L"Connecting to Pastral agent");
            HomeEmptyStateDetail().Text(
                L"Recent clips will appear after the local Health check completes.");
            break;
        case ConnectionState::Connected:
            HomeCaptureValue().Text(snapshot.synthetic ? L"Preview mode" : L"Connected");
            HomeEmptyStateTitle().Text(
                snapshot.synthetic ? L"No synthetic previews are available" : L"Recent history is not available yet");
            HomeEmptyStateDetail().Text(
                snapshot.synthetic
                    ? L"The Debug presentation provider returned no bounded preview records."
                    : L"The authenticated Health connection is active. Paged history IPC is not implemented in this build.");
            break;
        case ConnectionState::Disconnected:
            HomeCaptureValue().Text(L"Unavailable");
            HomeEmptyStateTitle().Text(L"Recent clips are unavailable");
            HomeEmptyStateDetail().Text(
                L"Start the local agent, then retry the authenticated connection.");
            break;
        case ConnectionState::CapturePaused:
            HomeCaptureValue().Text(L"Paused");
            HomeEmptyStateTitle().Text(L"Capture is paused");
            HomeEmptyStateDetail().Text(
                L"Resume capture from the agent before expecting new clipboard activity.");
            break;
        case ConnectionState::ProtocolMismatch:
            HomeCaptureValue().Text(L"Version mismatch");
            HomeEmptyStateTitle().Text(L"Recent clips are unavailable");
            HomeEmptyStateDetail().Text(
                L"Update the manager and agent to compatible versions, then retry.");
            break;
        case ConnectionState::Error:
            HomeCaptureValue().Text(L"Needs attention");
            HomeEmptyStateTitle().Text(L"Recent clips are unavailable");
            HomeEmptyStateDetail().Text(
                L"Resolve the manager status above before requesting clipboard history.");
            break;
        }

        HomeSyntheticNotice().IsOpen(snapshot.synthetic);
        HomeSyntheticNotice().Visibility(snapshot.synthetic ? Visibility::Visible : Visibility::Collapsed);
        auto const canRetry =
            snapshot.connection == ConnectionState::Disconnected ||
            snapshot.connection == ConnectionState::ProtocolMismatch ||
            snapshot.connection == ConnectionState::Error;
        auto const canRefresh = snapshot.connection == ConnectionState::Connected && !snapshot.synthetic;
        RetryConnectionButton().Content(winrt::box_value(
            canRetry ? L"Retry" : isLoading ? L"Checking…" : L"Refresh"));
        RetryConnectionButton().IsEnabled(!isLoading && (canRetry || canRefresh));
        RetryConnectionButton().Visibility(
            (isLoading || canRetry || canRefresh) ? Visibility::Visible : Visibility::Collapsed);

        m_recentClips.Clear();
        for (auto const& clip : snapshot.clips)
        {
            m_recentClips.Append(winrt::make<ClipPreviewViewModel>(clip));
        }

        auto const hasClips = m_recentClips.Size() > 0;
        HomeRecentClipsList().Visibility(hasClips ? Visibility::Visible : Visibility::Collapsed);
        HomeEmptyStatePanel().Visibility(hasClips ? Visibility::Collapsed : Visibility::Visible);
        HomeRecentCount().Text(winrt::hstring(std::to_wstring(m_recentClips.Size()) + L" items"));
    }
}
