#include "pch.h"
#include "App.xaml.h"
#include "MainWindow.xaml.h"

using namespace winrt;
using namespace Microsoft::UI::Xaml;

namespace winrt::Pastral::Manager::implementation
{
    App::App()
    {
        InitializeComponent();
    }

    void App::OnLaunched(LaunchActivatedEventArgs const&)
    {
        m_window = winrt::make<MainWindow>();
        m_window.Closed([weak = get_weak()](auto&&, auto&&)
        {
            if (auto app = weak.get())
            {
                app->m_window = nullptr;
            }
            Application::Current().Exit();
        });
        m_window.Activate();
    }
}
