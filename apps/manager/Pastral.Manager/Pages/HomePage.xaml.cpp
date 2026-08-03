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
        ApplySnapshot(m_provider->LoadSnapshot());
    }

    void HomePage::ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot const& snapshot)
    {
        using ::Pastral::Manager::Presentation::ConnectionState;
        using winrt::Microsoft::UI::Xaml::Controls::InfoBarSeverity;
        using winrt::Microsoft::UI::Xaml::Visibility;

        HomeConnectionStatus().Title(winrt::hstring(snapshot.statusTitle));
        HomeConnectionStatus().Message(winrt::hstring(snapshot.statusDetail));
        HomeConnectionStatus().Severity(
            snapshot.connection == ConnectionState::Error ||
            snapshot.connection == ConnectionState::ProtocolMismatch
                ? InfoBarSeverity::Error
                : InfoBarSeverity::Informational);
        HomeConnectionStatus().IsOpen(true);

        HomeStatusTitle().Text(winrt::hstring(snapshot.statusTitle));
        HomeStatusDetail().Text(winrt::hstring(snapshot.statusDetail));
        HomeProfileValue().Text(winrt::hstring(snapshot.activeProfile));
        HomeStorageValue().Text(winrt::hstring(snapshot.storageSummary));

        switch (snapshot.connection)
        {
        case ConnectionState::Loading:
            HomeCaptureValue().Text(L"Loading");
            break;
        case ConnectionState::Connected:
            HomeCaptureValue().Text(snapshot.synthetic ? L"Preview mode" : L"Connected");
            break;
        case ConnectionState::Disconnected:
            HomeCaptureValue().Text(L"Unavailable");
            break;
        case ConnectionState::CapturePaused:
            HomeCaptureValue().Text(L"Paused");
            break;
        case ConnectionState::ProtocolMismatch:
            HomeCaptureValue().Text(L"Version mismatch");
            break;
        case ConnectionState::Error:
            HomeCaptureValue().Text(L"Error");
            break;
        }

        HomeSyntheticNotice().IsOpen(snapshot.synthetic);
        HomeSyntheticNotice().Visibility(snapshot.synthetic ? Visibility::Visible : Visibility::Collapsed);
        RetryConnectionButton().Visibility(
            snapshot.connection == ConnectionState::Disconnected
                ? Visibility::Visible
                : Visibility::Collapsed);

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
