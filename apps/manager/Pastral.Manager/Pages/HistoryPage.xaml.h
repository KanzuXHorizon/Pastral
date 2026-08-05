#pragma once

#include "HistoryPage.g.h"
#include "../Services/ManagerDataProvider.h"
#include "../ViewModels/ClipPreviewViewModel.h"

#include <cstdint>

namespace winrt::Pastral::Manager::implementation
{
    struct HistoryPage : HistoryPageT<HistoryPage>
    {
        HistoryPage();

        void SearchBox_TextChanged(
            winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBox const& sender,
            winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBoxTextChangedEventArgs const& args);
        void ResultsList_SelectionChanged(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const& args);
        void ClearFilters_Click(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& args);
        void Retry_Click(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& args);
        void BackToResults_Click(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& args);
        void Page_SizeChanged(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::SizeChangedEventArgs const& args);
        void Page_Loaded(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& args);
        void Page_Unloaded(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& args);

    private:
        void LoadSnapshot();
        void RefreshSnapshot();
        void SearchSnapshot(std::wstring query);
        void ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot snapshot);
        void RefreshResults();
        void UpdateSelectionDetails();
        void ClearSelectionDetails();
        void BeginReadActivity(winrt::hstring const& announcement);
        void UpdateResponsiveLayout(bool restoreResultsFocus = false);

        std::shared_ptr<::Pastral::Manager::Presentation::IManagerDataProvider> m_provider;
        std::vector<::Pastral::Manager::Presentation::ClipPreviewData> m_allClips;
        ::Pastral::Manager::Presentation::ConnectionState m_connection{
            ::Pastral::Manager::Presentation::ConnectionState::Disconnected
        };
        winrt::Microsoft::UI::Dispatching::DispatcherQueueTimer m_searchTimer{ nullptr };
        bool m_synthetic{ false };
        bool m_hasMore{ false };
        bool m_unloaded{ false };
        bool m_hasLoadedOnce{ false };
        bool m_isWideLayout{ true };
        bool m_showingDetails{ false };
        bool m_suppressDetailTransition{ false };
        std::uint64_t m_loadGeneration{};
        winrt::Windows::Foundation::Collections::IObservableVector<
            winrt::Pastral::Manager::ClipPreviewViewModel> m_results{
                winrt::single_threaded_observable_vector<winrt::Pastral::Manager::ClipPreviewViewModel>()
            };
    };
}

namespace winrt::Pastral::Manager::factory_implementation
{
    struct HistoryPage : HistoryPageT<HistoryPage, implementation::HistoryPage>
    {
    };
}
