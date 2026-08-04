#pragma once

#include "../ViewModels/ManagerState.h"

#include <functional>
#include <string>

namespace Pastral::Manager::Presentation
{
    using SnapshotCompletion = std::function<void(ManagerSnapshot)>;

    struct IManagerDataProvider
    {
        virtual ~IManagerDataProvider() = default;
        virtual void LoadSnapshotAsync(SnapshotCompletion completion) = 0;
        virtual void RefreshAsync(SnapshotCompletion completion) = 0;
        virtual void SearchAsync(std::wstring query, SnapshotCompletion completion) = 0;
    };
}
