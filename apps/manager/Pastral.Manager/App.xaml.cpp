#include "pch.h"
#include "App.xaml.h"
#include "MainWindow.xaml.h"

using namespace winrt;
using namespace Microsoft::UI::Xaml;

namespace winrt::Pastral::Manager::implementation
{
    namespace
    {
#if defined(_DEBUG)
        void ConfigureDiagnosticLanguage()
        {
            constexpr wchar_t VariableName[] = L"PASTRAL_MANAGER_LANGUAGE";
            wchar_t value[16]{};
            constexpr DWORD ValueCapacity = static_cast<DWORD>(sizeof(value) / sizeof(value[0]));
            auto const length = GetEnvironmentVariableW(
                VariableName,
                value,
                ValueCapacity);
            if (length == 0 || length >= ValueCapacity)
            {
                return;
            }

            std::wstring_view const requested{ value, length };
            if (requested == L"en-US" || requested == L"vi-VN")
            {
                winrt::Microsoft::Windows::Globalization::ApplicationLanguages::PrimaryLanguageOverride(
                    winrt::hstring(requested));
            }
            else
            {
                return;
            }
        }
#endif
    }

    App::App()
    {
#if defined(_DEBUG)
        ConfigureDiagnosticLanguage();
#endif
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
