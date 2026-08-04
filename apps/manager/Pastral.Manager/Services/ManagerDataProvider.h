#pragma once

#include "IManagerDataProvider.h"

#include <memory>

namespace Pastral::Manager::Presentation
{
    [[nodiscard]] ManagerSnapshot CreateLoadingSnapshot();
    [[nodiscard]] std::shared_ptr<IManagerDataProvider> CreateManagerDataProvider();
}
