#pragma once

#include "HistoryPage.g.h"

namespace winrt::Pastral::Manager::implementation
{
    struct HistoryPage : HistoryPageT<HistoryPage>
    {
        HistoryPage();
    };
}

namespace winrt::Pastral::Manager::factory_implementation
{
    struct HistoryPage : HistoryPageT<HistoryPage, implementation::HistoryPage>
    {
    };
}
