#include "pch.h"
#include "HistoryPage.xaml.h"

#if __has_include("HistoryPage.g.cpp")
#include "HistoryPage.g.cpp"
#endif

namespace winrt::Pastral::Manager::implementation
{
    HistoryPage::HistoryPage()
    {
        InitializeComponent();
    }
}
