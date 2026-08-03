#pragma once

#include "HistoryPage.g.h"
#include "../Services/ManagerDataProvider.h"
#include "../ViewModels/ClipPreviewViewModel.h"

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

    private:
        void LoadSnapshot();
        void ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot snapshot);
        void RefreshResults();
        void UpdateSelectionDetails();
        void ClearSelectionDetails();

        std::shared_ptr<::Pastral::Manager::Presentation::IManagerDataProvider> m_provider;
        std::vector<::Pastral::Manager::Presentation::ClipPreviewData> m_allClips;
        ::Pastral::Manager::Presentation::ConnectionState m_connection{
            ::Pastral::Manager::Presentation::ConnectionState::Disconnected
        };
        bool m_synthetic{ false };
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
