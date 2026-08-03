#include "pch.h"
#include "MainWindow.xaml.h"
#include "Pages/HomePage.xaml.h"
#include "Pages/HistoryPage.xaml.h"

#if __has_include("MainWindow.g.cpp")
#include "MainWindow.g.cpp"
#endif

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Controls;

namespace winrt::Pastral::Manager::implementation
{
    MainWindow::MainWindow()
    {
        InitializeComponent();
        ExtendsContentIntoTitleBar(true);
        SetTitleBar(AppTitleBar());

        if (ShellNavigationView().MenuItems().Size() > 0)
        {
            ShellNavigationView().SelectedItem(ShellNavigationView().MenuItems().GetAt(0));
        }
        NavigateTo(L"home");
    }

    void MainWindow::ShellNavigationView_SelectionChanged(
        NavigationView const&,
        NavigationViewSelectionChangedEventArgs const& args)
    {
        auto item = args.SelectedItem().try_as<NavigationViewItem>();
        if (!item || !item.Tag())
        {
            ShowNavigationError();
            return;
        }

        NavigateTo(unbox_value<hstring>(item.Tag()));
    }

    void MainWindow::NavigateTo(std::wstring_view tag)
    {
        if (tag == L"home")
        {
            ContentFrame().Navigate(xaml_typename<Pastral::Manager::HomePage>());
            return;
        }
        if (tag == L"history")
        {
            ContentFrame().Navigate(xaml_typename<Pastral::Manager::HistoryPage>());
            return;
        }
        ShowNavigationError();
    }

    void MainWindow::ShowNavigationError()
    {
        GlobalStatusBar().Severity(InfoBarSeverity::Error);
        GlobalStatusBar().Title(L"Navigation unavailable");
        GlobalStatusBar().Message(L"The requested manager page is not part of this verified build.");
        GlobalStatusBar().IsOpen(true);
    }
}
