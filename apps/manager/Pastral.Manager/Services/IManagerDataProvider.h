#pragma once

#include "../ViewModels/ManagerState.h"

namespace Pastral::Manager::Presentation
{
    struct IManagerDataProvider
    {
        virtual ~IManagerDataProvider() = default;
        virtual ManagerSnapshot LoadSnapshot() const = 0;
    };
}
