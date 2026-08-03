#pragma once

#include "HomePage.g.h"
#include "../Services/ManagerDataProvider.h"
#include "../ViewModels/ClipPreviewViewModel.h"

namespace winrt::Pastral::Manager::implementation
{
    struct HomePage : HomePageT<HomePage>
    {
        HomePage();

        void RetryConnection_Click(
            winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& args);

    private:
        void LoadSnapshot();
        void ApplySnapshot(::Pastral::Manager::Presentation::ManagerSnapshot const& snapshot);

        std::shared_ptr<::Pastral::Manager::Presentation::IManagerDataProvider> m_provider;
        winrt::Windows::Foundation::Collections::IObservableVector<
            winrt::Pastral::Manager::ClipPreviewViewModel> m_recentClips{
                winrt::single_threaded_observable_vector<winrt::Pastral::Manager::ClipPreviewViewModel>()
            };
    };
}

namespace winrt::Pastral::Manager::factory_implementation
{
    struct HomePage : HomePageT<HomePage, implementation::HomePage>
    {
    };
}
