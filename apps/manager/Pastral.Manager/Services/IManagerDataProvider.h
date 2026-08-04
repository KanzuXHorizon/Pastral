#pragma once

#include "../ViewModels/ManagerState.h"

#include <functional>

namespace Pastral::Manager::Presentation
{
    using SnapshotCompletion = std::function<void(ManagerSnapshot)>;

    struct IManagerDataProvider
    {
        virtual ~IManagerDataProvider() = default;
        virtual void LoadSnapshotAsync(SnapshotCompletion completion) = 0;
    };
}
