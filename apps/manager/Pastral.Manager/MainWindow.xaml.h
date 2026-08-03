#pragma once

#include "MainWindow.g.h"

namespace winrt::Pastral::Manager::implementation
{
    struct MainWindow : MainWindowT<MainWindow>
    {
        MainWindow();

        void ShellNavigationView_SelectionChanged(
            winrt::Microsoft::UI::Xaml::Controls::NavigationView const& sender,
            winrt::Microsoft::UI::Xaml::Controls::NavigationViewSelectionChangedEventArgs const& args);

    private:
        void NavigateTo(std::wstring_view tag);
        void ShowNavigationError();
    };
}

namespace winrt::Pastral::Manager::factory_implementation
{
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow>
    {
    };
}
