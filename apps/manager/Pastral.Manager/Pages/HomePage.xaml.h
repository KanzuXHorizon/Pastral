#pragma once

#include "HomePage.g.h"

namespace winrt::Pastral::Manager::implementation
{
    struct HomePage : HomePageT<HomePage>
    {
        HomePage();
    };
}

namespace winrt::Pastral::Manager::factory_implementation
{
    struct HomePage : HomePageT<HomePage, implementation::HomePage>
    {
    };
}
